use core::convert::Infallible;
use core::marker::PhantomData;
use core::num::NonZeroUsize;
use core::sync::atomic::{AtomicBool, Ordering};

use super::root_context::RootContext;
use crate::api::guardian::AlwaysRestart;
use crate::runtime::mailbox::traits::{ActorRuntime, MailboxPair};
use crate::runtime::mailbox::{MailboxOptions, PriorityMailboxSpawnerHandle};
use crate::runtime::message::DynMessage;
use crate::runtime::metrics::MetricsSinkShared;
use crate::runtime::scheduler::receive_timeout::NoopReceiveTimeoutDriver;
use crate::runtime::scheduler::{ReadyQueueWorker, SchedulerBuilder};
use crate::runtime::system::{InternalActorSystem, InternalActorSystemSettings};
use crate::serializer_extension_id;
use crate::{
  default_failure_telemetry, Extension, ExtensionId, Extensions, FailureEventHandler, FailureEventListener,
  FailureEventStream, FailureTelemetryBuilderShared, FailureTelemetryShared, MailboxRuntime, PriorityEnvelope,
  SerializerRegistryExtension, TelemetryContext, TelemetryObservationConfig,
};
use crate::{ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared};
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, QueueError};

/// Primary instance of the actor system.
///
/// Responsible for actor spawning, management, and message dispatching.
pub struct ActorSystem<U, R, Strat = AlwaysRestart>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, R>, {
  inner: InternalActorSystem<DynMessage, R, Strat>,
  shutdown: ShutdownToken,
  extensions: Extensions,
  ready_queue_worker_count: NonZeroUsize,
  _marker: PhantomData<U>,
}

/// Builder that constructs an [`ActorSystem`] by applying configuration overrides on top of a runtime preset.
pub struct ActorSystemBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  runtime: R,
  config: ActorSystemConfig<R>,
  _marker: PhantomData<U>,
}

impl<U, R> ActorSystemBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new builder with default configuration.
  #[must_use]
  pub fn new(runtime: R) -> Self {
    Self {
      runtime,
      config: ActorSystemConfig::default(),
      _marker: PhantomData,
    }
  }

  /// Returns a reference to the runtime preset owned by the builder.
  #[must_use]
  pub fn runtime(&self) -> &R {
    &self.runtime
  }

  /// Returns a mutable reference to the runtime preset.
  pub fn runtime_mut(&mut self) -> &mut R {
    &mut self.runtime
  }

  /// Returns a reference to the configuration being accumulated.
  #[must_use]
  pub fn config(&self) -> &ActorSystemConfig<R> {
    &self.config
  }

  /// Returns a mutable reference to the configuration being accumulated.
  pub fn config_mut(&mut self) -> &mut ActorSystemConfig<R> {
    &mut self.config
  }

  /// Replaces the configuration with the provided value.
  #[must_use]
  pub fn with_config(mut self, config: ActorSystemConfig<R>) -> Self {
    self.config = config;
    self
  }

  /// Applies in-place configuration updates using the given closure.
  #[must_use]
  pub fn configure<F>(mut self, configure: F) -> Self
  where
    F: FnOnce(&mut ActorSystemConfig<R>), {
    configure(&mut self.config);
    self
  }

  /// Consumes the builder and constructs an [`ActorSystem`].
  #[must_use]
  pub fn build(self) -> ActorSystem<U, R> {
    ActorSystem::new_with_runtime(self.runtime, self.config)
  }
}

#[derive(Clone)]
pub(crate) struct GenericActorRuntimeState<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  mailbox_runtime: ArcShared<R>,
  scheduler_builder: ArcShared<SchedulerBuilder<DynMessage, GenericActorRuntime<R>>>,
}

impl<R> GenericActorRuntimeState<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  #[must_use]
  pub(crate) fn new(actor_runtime: R) -> Self {
    Self {
      mailbox_runtime: ArcShared::new(actor_runtime),
      scheduler_builder: ArcShared::new(SchedulerBuilder::<DynMessage, GenericActorRuntime<R>>::ready_queue()),
    }
  }

  #[must_use]
  pub(crate) fn mailbox_runtime(&self) -> &R {
    &self.mailbox_runtime
  }

  #[must_use]
  pub(crate) fn mailbox_runtime_shared(&self) -> ArcShared<R> {
    self.mailbox_runtime.clone()
  }

  #[must_use]
  pub(crate) fn into_mailbox_runtime(self) -> R {
    self
      .mailbox_runtime
      .try_unwrap()
      .unwrap_or_else(|shared| (*shared).clone())
  }

  #[must_use]
  pub(crate) fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, GenericActorRuntime<R>>> {
    self.scheduler_builder.clone()
  }

  pub(crate) fn set_scheduler_builder(&mut self, builder: ArcShared<SchedulerBuilder<DynMessage, GenericActorRuntime<R>>>) {
    self.scheduler_builder = builder;
  }
}

/// Shared handle used to expose mailbox construction capabilities to the scheduler layer without
/// leaking the underlying mailbox factory implementation.
#[derive(Clone)]
pub struct MailboxHandleFactoryStub<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  factory: ArcShared<R>,
  metrics_sink: Option<MetricsSinkShared>,
}

impl<R> MailboxHandleFactoryStub<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  #[must_use]
  pub(crate) fn new(factory: ArcShared<R>) -> Self {
    Self {
      factory,
      metrics_sink: None,
    }
  }

  /// Creates a stub by cloning the provided runtime and wrapping it in a shared handle.
  #[must_use]
  pub fn from_runtime(runtime: R) -> Self
  where
    R: Clone, {
    Self::new(ArcShared::new(runtime))
  }

  /// Returns a priority mailbox spawner for the given message type using the stored factory.
  #[must_use]
  pub fn priority_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, R>
  where
    M: Element,
    R::Queue<PriorityEnvelope<M>>: Clone,
    R::Signal: Clone, {
    PriorityMailboxSpawnerHandle::new(self.factory.clone()).with_metrics_sink(self.metrics_sink.clone())
  }

  /// Returns a new stub with the provided metrics sink.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Mutable setter for the metrics sink.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }
}

/// Bundle that contains runtime-dependent components required by [`ActorSystem`].
///
/// This lightweight container currently stores only the mailbox factory, but it is
/// designed to host scheduler builders, timeout drivers, and other platform-specific
/// elements in future iterations.
#[derive(Clone)]
pub struct GenericActorRuntime<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  core: GenericActorRuntimeState<R>,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, GenericActorRuntime<R>>>,
  receive_timeout_driver: Option<ReceiveTimeoutDriverShared<R>>,
  root_event_listener: Option<FailureEventListener>,
  root_escalation_handler: Option<FailureEventHandler>,
  metrics_sink: Option<MetricsSinkShared>,
}

impl<R> GenericActorRuntime<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new runtime bundle with the provided mailbox factory.
  #[must_use]
  pub fn new(actor_runtime: R) -> Self {
    Self {
      core: GenericActorRuntimeState::new(actor_runtime),
      receive_timeout_factory: None,
      receive_timeout_driver: Some(ReceiveTimeoutDriverShared::new(NoopReceiveTimeoutDriver::default())),
      root_event_listener: None,
      root_escalation_handler: None,
      metrics_sink: None,
    }
  }

  /// Returns a shared reference to the mailbox factory.
  #[must_use]
  pub fn mailbox_runtime(&self) -> &R {
    self.core.mailbox_runtime()
  }

  /// Consumes the bundle and returns the mailbox factory.
  #[must_use]
  pub fn into_mailbox_runtime(self) -> R {
    let Self { core, .. } = self;
    core.into_mailbox_runtime()
  }

  /// Returns the shared mailbox factory handle.
  #[must_use]
  pub fn mailbox_runtime_shared(&self) -> ArcShared<R> {
    self.core.mailbox_runtime_shared()
  }

  /// Returns the receive-timeout factory configured for this bundle.
  #[must_use]
  pub fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, GenericActorRuntime<R>>> {
    self.receive_timeout_factory.clone()
  }

  /// Returns the receive-timeout driver configured for this bundle.
  #[must_use]
  pub fn receive_timeout_driver(&self) -> Option<ReceiveTimeoutDriverShared<R>> {
    self.receive_timeout_driver.clone()
  }

  /// Sets the receive-timeout factory using the base mailbox factory type.
  #[must_use]
  pub fn with_receive_timeout_factory(mut self, factory: ReceiveTimeoutFactoryShared<DynMessage, R>) -> Self {
    self.receive_timeout_factory = Some(factory.for_runtime_bundle());
    self
  }

  /// Sets the receive-timeout factory using a bundle-ready factory.
  #[must_use]
  pub fn with_receive_timeout_factory_shared(
    mut self,
    factory: ReceiveTimeoutFactoryShared<DynMessage, GenericActorRuntime<R>>,
  ) -> Self {
    self.receive_timeout_factory = Some(factory);
    self
  }

  /// Overrides the receive-timeout driver.
  #[must_use]
  pub fn with_receive_timeout_driver(mut self, driver: Option<ReceiveTimeoutDriverShared<R>>) -> Self {
    self.receive_timeout_driver = driver;
    self
  }

  /// Mutably overrides the receive-timeout driver.
  pub fn set_receive_timeout_driver(&mut self, driver: Option<ReceiveTimeoutDriverShared<R>>) {
    self.receive_timeout_driver = driver;
  }

  /// Returns a factory built by the configured receive-timeout driver, if any.
  #[must_use]
  pub fn receive_timeout_driver_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, GenericActorRuntime<R>>> {
    self
      .receive_timeout_driver
      .as_ref()
      .map(|driver| driver.build_factory())
  }

  /// Returns the root failure event listener configured for the bundle.
  #[must_use]
  pub fn root_event_listener(&self) -> Option<FailureEventListener> {
    self.root_event_listener.clone()
  }

  /// Overrides the root failure event listener.
  #[must_use]
  pub fn with_root_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.root_event_listener = listener;
    self
  }

  /// Returns the root escalation handler configured for the bundle.
  #[must_use]
  pub fn root_escalation_handler(&self) -> Option<FailureEventHandler> {
    self.root_escalation_handler.clone()
  }

  /// Overrides the root escalation handler.
  #[must_use]
  pub fn with_root_escalation_handler(mut self, handler: Option<FailureEventHandler>) -> Self {
    self.root_escalation_handler = handler;
    self
  }

  /// Returns the metrics sink configured for this bundle.
  #[must_use]
  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  /// Overrides the metrics sink.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Sets the metrics sink using a concrete shared handle.
  #[must_use]
  pub fn with_metrics_sink_shared(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics_sink = Some(sink);
    self
  }

  /// Returns a handle that can spawn priority mailboxes without exposing the factory implementation.
  #[must_use]
  pub fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, GenericActorRuntime<R>>
  where
    M: Element,
    R::Queue<PriorityEnvelope<M>>: Clone,
    R::Signal: Clone, {
    MailboxHandleFactoryStub::from_runtime(self.clone())
      .with_metrics_sink(self.metrics_sink.clone())
      .priority_spawner()
  }

  /// Overrides the scheduler builder used when constructing the actor system.
  #[must_use]
  pub fn with_scheduler_builder(mut self, builder: SchedulerBuilder<DynMessage, GenericActorRuntime<R>>) -> Self {
    self.core.set_scheduler_builder(ArcShared::new(builder));
    self
  }

  /// Overrides the scheduler builder using a pre-wrapped shared handle.
  #[must_use]
  pub fn with_scheduler_builder_shared(
    mut self,
    builder: ArcShared<SchedulerBuilder<DynMessage, GenericActorRuntime<R>>>,
  ) -> Self {
    self.core.set_scheduler_builder(builder);
    self
  }

  /// Returns the scheduler builder configured for this runtime bundle.
  #[must_use]
  pub fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, GenericActorRuntime<R>>> {
    self.core.scheduler_builder()
  }
}

impl<R> MailboxRuntime for GenericActorRuntime<R>
where
  R: MailboxRuntime + Clone + 'static,
{
  type Concurrency = R::Concurrency;
  type Mailbox<M>
    = R::Mailbox<M>
  where
    M: Element;
  type Producer<M>
    = R::Producer<M>
  where
    M: Element;
  type Queue<M>
    = R::Queue<M>
  where
    M: Element;
  type Signal = R::Signal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    self.mailbox_runtime().build_mailbox(options)
  }
}

impl<R> ActorRuntime for GenericActorRuntime<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  type Base = R;

  fn mailbox_runtime(&self) -> &Self::Base {
    GenericActorRuntime::mailbox_runtime(self)
  }

  fn into_mailbox_runtime(self) -> Self::Base {
    GenericActorRuntime::into_mailbox_runtime(self)
  }

  fn mailbox_runtime_shared(&self) -> ArcShared<Self::Base> {
    GenericActorRuntime::mailbox_runtime_shared(self)
  }

  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self>> {
    GenericActorRuntime::receive_timeout_factory(self)
  }

  fn receive_timeout_driver(&self) -> Option<ReceiveTimeoutDriverShared<Self::Base>> {
    GenericActorRuntime::receive_timeout_driver(self)
  }

  fn with_receive_timeout_factory(self, factory: ReceiveTimeoutFactoryShared<DynMessage, Self::Base>) -> Self {
    GenericActorRuntime::with_receive_timeout_factory(self, factory)
  }

  fn with_receive_timeout_factory_shared(self, factory: ReceiveTimeoutFactoryShared<DynMessage, Self>) -> Self {
    GenericActorRuntime::with_receive_timeout_factory_shared(self, factory)
  }

  fn with_receive_timeout_driver(self, driver: Option<ReceiveTimeoutDriverShared<Self::Base>>) -> Self {
    GenericActorRuntime::with_receive_timeout_driver(self, driver)
  }

  fn set_receive_timeout_driver(&mut self, driver: Option<ReceiveTimeoutDriverShared<Self::Base>>) {
    GenericActorRuntime::set_receive_timeout_driver(self, driver);
  }

  fn receive_timeout_driver_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self>> {
    GenericActorRuntime::receive_timeout_driver_factory(self)
  }

  fn root_event_listener(&self) -> Option<FailureEventListener> {
    GenericActorRuntime::root_event_listener(self)
  }

  fn with_root_event_listener(self, listener: Option<FailureEventListener>) -> Self {
    GenericActorRuntime::with_root_event_listener(self, listener)
  }

  fn root_escalation_handler(&self) -> Option<FailureEventHandler> {
    GenericActorRuntime::root_escalation_handler(self)
  }

  fn with_root_escalation_handler(self, handler: Option<FailureEventHandler>) -> Self {
    GenericActorRuntime::with_root_escalation_handler(self, handler)
  }

  fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    GenericActorRuntime::metrics_sink(self)
  }

  fn with_metrics_sink(self, sink: Option<MetricsSinkShared>) -> Self {
    GenericActorRuntime::with_metrics_sink(self, sink)
  }

  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self {
    GenericActorRuntime::with_metrics_sink_shared(self, sink)
  }

  fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, Self>
  where
    M: Element,
    <Self::Base as crate::MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
    <Self::Base as crate::MailboxRuntime>::Signal: Clone, {
    GenericActorRuntime::priority_mailbox_spawner(self)
  }

  fn with_scheduler_builder(self, builder: SchedulerBuilder<DynMessage, Self>) -> Self {
    GenericActorRuntime::with_scheduler_builder(self, builder)
  }

  fn with_scheduler_builder_shared(self, builder: ArcShared<SchedulerBuilder<DynMessage, Self>>) -> Self {
    GenericActorRuntime::with_scheduler_builder_shared(self, builder)
  }

  fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, Self>> {
    GenericActorRuntime::scheduler_builder(self)
  }
}

/// Configuration options applied when constructing an [`ActorSystem`].
pub struct ActorSystemConfig<R>
where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  /// Listener invoked when failures bubble up to the root guardian.
  failure_event_listener: Option<FailureEventListener>,
  /// Receive-timeout scheduler factory used by all actors spawned in the system.
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, R>>,
  /// Metrics sink shared across the actor runtime.
  metrics_sink: Option<MetricsSinkShared>,
  /// Telemetry invoked when failures reach the root guardian.
  failure_telemetry: Option<FailureTelemetryShared>,
  /// Builder used to create telemetry implementations。
  failure_telemetry_builder: Option<FailureTelemetryBuilderShared>,
  /// Observation configuration applied to telemetry calls。
  failure_observation_config: Option<TelemetryObservationConfig>,
  /// Extension registry configured for the actor system.
  extensions: Extensions,
  /// Default ReadyQueue worker count supplied by configuration.
  ready_queue_worker_count: Option<NonZeroUsize>,
}

impl<R> Default for ActorSystemConfig<R>
where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn default() -> Self {
    Self {
      failure_event_listener: None,
      receive_timeout_factory: None,
      metrics_sink: None,
      failure_telemetry: None,
      failure_telemetry_builder: None,
      failure_observation_config: None,
      extensions: Extensions::new(),
      ready_queue_worker_count: None,
    }
  }
}

impl<R> ActorSystemConfig<R>
where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Sets the failure event listener.
  pub fn with_failure_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.failure_event_listener = listener;
    self
  }

  /// Sets the receive-timeout factory.
  pub fn with_receive_timeout_factory(mut self, factory: Option<ReceiveTimeoutFactoryShared<DynMessage, R>>) -> Self {
    self.receive_timeout_factory = factory;
    self
  }

  /// Sets the metrics sink.
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Sets the failure telemetry implementation.
  pub fn with_failure_telemetry(mut self, telemetry: Option<FailureTelemetryShared>) -> Self {
    self.failure_telemetry = telemetry;
    self
  }

  /// Sets the failure telemetry builder implementation.
  pub fn with_failure_telemetry_builder(mut self, builder: Option<FailureTelemetryBuilderShared>) -> Self {
    self.failure_telemetry_builder = builder;
    self
  }

  /// Sets telemetry observation configuration.
  pub fn with_failure_observation_config(mut self, config: Option<TelemetryObservationConfig>) -> Self {
    self.failure_observation_config = config;
    self
  }

  /// Sets the default ReadyQueue worker count.
  pub fn with_ready_queue_worker_count(mut self, worker_count: Option<NonZeroUsize>) -> Self {
    self.ready_queue_worker_count = worker_count;
    self
  }

  /// Sets the metrics sink using a concrete shared handle.
  #[must_use]
  pub fn with_metrics_sink_shared(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics_sink = Some(sink);
    self
  }

  /// Mutable setter for the failure event listener.
  pub fn set_failure_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.failure_event_listener = listener;
  }

  /// Mutable setter for the receive-timeout factory.
  pub fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<DynMessage, R>>) {
    self.receive_timeout_factory = factory;
  }

  /// Mutable setter for the metrics sink.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }

  /// Mutable setter for the failure telemetry implementation.
  pub fn set_failure_telemetry(&mut self, telemetry: Option<FailureTelemetryShared>) {
    self.failure_telemetry = telemetry;
  }

  /// Mutable setter for the failure telemetry builder.
  pub fn set_failure_telemetry_builder(&mut self, builder: Option<FailureTelemetryBuilderShared>) {
    self.failure_telemetry_builder = builder;
  }

  /// Mutable setter for telemetry observation config.
  pub fn set_failure_observation_config(&mut self, config: Option<TelemetryObservationConfig>) {
    self.failure_observation_config = config;
  }

  /// Mutable setter for the default ReadyQueue worker count.
  pub fn set_ready_queue_worker_count(&mut self, worker_count: Option<NonZeroUsize>) {
    self.ready_queue_worker_count = worker_count;
  }

  pub(crate) fn failure_event_listener(&self) -> Option<FailureEventListener> {
    self.failure_event_listener.clone()
  }

  pub(crate) fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, R>> {
    self.receive_timeout_factory.clone()
  }

  pub(crate) fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  pub(crate) fn failure_telemetry(&self) -> Option<FailureTelemetryShared> {
    self.failure_telemetry.clone()
  }

  pub(crate) fn failure_telemetry_builder(&self) -> Option<FailureTelemetryBuilderShared> {
    self.failure_telemetry_builder.clone()
  }

  pub(crate) fn failure_observation_config(&self) -> Option<TelemetryObservationConfig> {
    self.failure_observation_config.clone()
  }

  pub(crate) fn ready_queue_worker_count(&self) -> Option<NonZeroUsize> {
    self.ready_queue_worker_count
  }

  /// Replaces the extension registry in the configuration.
  pub fn with_extensions(mut self, extensions: Extensions) -> Self {
    self.extensions = extensions;
    self
  }

  /// Registers an extension handle in the configuration.
  pub fn with_extension_handle<E>(self, extension: ArcShared<E>) -> Self
  where
    E: Extension + 'static, {
    let extensions = &self.extensions;
    extensions.register(extension);
    self
  }

  /// Registers an extension value in the configuration by wrapping it with `ArcShared`.
  pub fn with_extension_value<E>(self, extension: E) -> Self
  where
    E: Extension + 'static, {
    self.with_extension_handle(ArcShared::new(extension))
  }

  /// Returns the registered extensions.
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Mutably overrides the extensions registry.
  pub fn set_extensions(&mut self, extensions: Extensions) {
    self.extensions = extensions;
  }

  /// Registers an extension on the existing registry.
  pub fn register_extension<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static, {
    self.extensions.register(extension);
  }

  /// Registers a dynamically typed extension on the existing registry.
  pub fn register_extension_dyn(&self, extension: ArcShared<dyn Extension>) {
    self.extensions.register_dyn(extension);
  }
}

/// Execution runner for the actor system.
///
/// Wraps `ActorSystem` and provides an interface for execution on an asynchronous runtime.
pub struct ActorSystemRunner<U, R, Strat = AlwaysRestart>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, R>, {
  system: ActorSystem<U, R, Strat>,
  ready_queue_worker_count: NonZeroUsize,
  _marker: PhantomData<U>,
}

impl<U, R> ActorSystem<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new actor system with an explicit runtime and configuration.
  pub fn new_with_runtime(runtime: R, config: ActorSystemConfig<R>) -> Self {
    let receive_timeout_from_runtime = runtime.receive_timeout_factory();
    let root_listener_from_runtime = runtime.root_event_listener();
    let root_handler_from_runtime = runtime.root_escalation_handler();
    let metrics_from_runtime = runtime.metrics_sink();
    let scheduler_builder = runtime.scheduler_builder();

    let extensions_handle = config.extensions();
    if extensions_handle.get(serializer_extension_id()).is_none() {
      let extension = ArcShared::new(SerializerRegistryExtension::new());
      extensions_handle.register(extension);
    }
    let extensions = extensions_handle.clone();

    let receive_timeout_factory = config
      .receive_timeout_factory()
      .or(receive_timeout_from_runtime)
      .or_else(|| runtime.receive_timeout_driver_factory());
    let root_event_listener = config.failure_event_listener().or(root_listener_from_runtime);
    let metrics_sink = config.metrics_sink().or(metrics_from_runtime);
    let telemetry_builder = config.failure_telemetry_builder();
    let root_failure_telemetry = if let Some(builder) = telemetry_builder.clone() {
      let ctx = TelemetryContext::new(metrics_sink.clone(), extensions.clone());
      builder.build(&ctx)
    } else {
      config.failure_telemetry().unwrap_or_else(default_failure_telemetry)
    };

    let mut observation_config = config
      .failure_observation_config()
      .unwrap_or_else(TelemetryObservationConfig::new);
    if let Some(sink) = metrics_sink.clone() {
      if observation_config.metrics_sink().is_none() {
        observation_config.set_metrics_sink(Some(sink));
      }
      #[cfg(feature = "std")]
      {
        if !observation_config.should_record_timing() {
          observation_config.set_record_timing(true);
        }
      }
    }

    let settings = InternalActorSystemSettings {
      root_event_listener,
      root_escalation_handler: root_handler_from_runtime,
      receive_timeout_factory,
      metrics_sink,
      root_failure_telemetry,
      root_observation_config: observation_config,
      extensions: extensions.clone(),
    };

    let ready_queue_worker_count = config
      .ready_queue_worker_count()
      .unwrap_or_else(|| NonZeroUsize::new(1).expect("ReadyQueue worker count must be non-zero"));

    Self {
      inner: InternalActorSystem::new_with_settings_and_builder(runtime, &scheduler_builder, settings),
      shutdown: ShutdownToken::default(),
      extensions,
      ready_queue_worker_count,
      _marker: PhantomData,
    }
  }

  /// Creates an actor system using the provided runtime and failure event stream.
  pub fn new_with_runtime_and_event_stream<E>(runtime: R, event_stream: &E) -> Self
  where
    E: FailureEventStream, {
    let config = ActorSystemConfig::default().with_failure_event_listener(Some(event_stream.listener()));
    Self::new_with_runtime(runtime, config)
  }

  /// Returns a builder that creates an actor system from the provided runtime preset.
  #[must_use]
  pub fn builder(runtime: R) -> ActorSystemBuilder<U, R> {
    ActorSystemBuilder::new(runtime)
  }
}

impl<U, B> ActorSystem<U, GenericActorRuntime<B>>
where
  U: Element,
  B: MailboxRuntime + Clone + 'static,
  B::Queue<PriorityEnvelope<DynMessage>>: Clone,
  B::Signal: Clone,
{
  /// Creates a new actor system from a mailbox runtime by wrapping it in [`GenericActorRuntime`].
  pub fn new(mailbox_runtime: B) -> Self {
    Self::new_with_runtime(GenericActorRuntime::new(mailbox_runtime), ActorSystemConfig::default())
  }

  /// Creates a new actor system with explicit configuration from a mailbox runtime.
  pub fn new_with_config(mailbox_runtime: B, config: ActorSystemConfig<GenericActorRuntime<B>>) -> Self {
    Self::new_with_runtime(GenericActorRuntime::new(mailbox_runtime), config)
  }
}

impl<U, R, Strat> ActorSystem<U, R, Strat>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, R>,
{
  /// Gets the shutdown token.
  ///
  /// # Returns
  /// Clone of the shutdown token
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.shutdown.clone()
  }

  /// Converts this system into a runner.
  ///
  /// The runner provides an interface suitable for execution on an asynchronous runtime.
  ///
  /// # Returns
  /// Actor system runner
  pub fn into_runner(self) -> ActorSystemRunner<U, R, Strat> {
    let ready_queue_worker_count = self.ready_queue_worker_count;
    ActorSystemRunner {
      system: self,
      ready_queue_worker_count,
      _marker: PhantomData,
    }
  }

  /// Gets the root context.
  ///
  /// The root context is used to spawn actors at the top level of the actor system.
  ///
  /// # Returns
  /// Mutable reference to the root context
  pub fn root_context(&mut self) -> RootContext<'_, U, R, Strat> {
    RootContext {
      inner: self.inner.root_context(),
      _marker: PhantomData,
    }
  }

  /// Returns a clone of the shared extension registry.
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Applies a closure to the specified extension and returns its result.
  pub fn extension<E, F, T>(&self, id: ExtensionId, f: F) -> Option<T>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> T, {
    self.extensions.with::<E, _, _>(id, f)
  }

  /// Registers an extension handle with the running actor system.
  pub fn register_extension<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static, {
    self.extensions.register(extension);
  }

  /// Registers a dynamically typed extension handle with the running actor system.
  pub fn register_extension_dyn(&self, extension: ArcShared<dyn Extension>) {
    self.extensions.register_dyn(extension);
  }

  /// Registers an extension value by wrapping it with `ArcShared`.
  pub fn register_extension_value<E>(&self, extension: E)
  where
    E: Extension + 'static, {
    self.register_extension(ArcShared::new(extension));
  }

  /// Executes message dispatching until the specified condition is met.
  ///
  /// # Arguments
  /// * `should_continue` - Closure that determines continuation condition. Continues execution while it returns `true`
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    F: FnMut() -> bool, {
    self.inner.run_until(should_continue).await
  }

  /// Executes message dispatching permanently.
  ///
  /// This function does not terminate normally. Returns only on error.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.run_forever().await
  }

  /// Dispatches one next message.
  ///
  /// Waits until a new message arrives if the queue is empty.
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.dispatch_next().await
  }

  /// Synchronously processes messages accumulated in the Ready queue, repeating until empty.
  /// Does not wait for new messages to arrive.
  pub fn run_until_idle(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let shutdown = self.shutdown.clone();
    self.inner.run_until_idle(|| !shutdown.is_triggered())
  }

  /// Returns a ReadyQueue worker handle if supported by the underlying scheduler.
  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<DynMessage, R>>> {
    self.inner.ready_queue_worker()
  }

  /// Indicates whether the scheduler supports ReadyQueue-based execution.
  #[must_use]
  pub fn supports_ready_queue(&self) -> bool {
    self.ready_queue_worker().is_some()
  }
}

impl<U, R, Strat> ActorSystemRunner<U, R, Strat>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, R>,
{
  /// Gets the number of ReadyQueue workers to spawn when driving the system.
  #[must_use]
  pub fn ready_queue_worker_count(&self) -> NonZeroUsize {
    self.ready_queue_worker_count
  }

  /// Updates the ReadyQueue worker count in place.
  pub fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize) {
    self.ready_queue_worker_count = worker_count;
  }

  /// Returns a new runner with the specified ReadyQueue worker count.
  #[must_use]
  pub fn with_ready_queue_worker_count(mut self, worker_count: NonZeroUsize) -> Self {
    self.set_ready_queue_worker_count(worker_count);
    self
  }

  /// Returns a ReadyQueue worker handle if supported by the underlying scheduler.
  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<DynMessage, R>>> {
    self.system.ready_queue_worker()
  }

  /// Indicates whether the scheduler supports ReadyQueue-based execution.
  #[must_use]
  pub fn supports_ready_queue(&self) -> bool {
    self.system.supports_ready_queue()
  }

  /// Gets the shutdown token.
  ///
  /// # Returns
  /// Clone of the shutdown token
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.system.shutdown.clone()
  }

  /// Executes message dispatching permanently.
  ///
  /// This function does not terminate normally. Returns only on error.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  pub async fn run_forever(mut self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.system.run_forever().await
  }

  /// Executes the runner as a Future.
  ///
  /// Alias for `run_forever`. Provides a name suitable for execution on an asynchronous runtime.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  pub async fn into_future(self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.run_forever().await
  }

  /// Extracts the internal actor system from the runner.
  ///
  /// # Returns
  /// Internal actor system
  pub fn into_inner(self) -> ActorSystem<U, R, Strat> {
    self.system
  }
}

/// Token that controls shutdown of the actor system.
///
/// Can be shared among multiple threads or tasks and cooperatively manages shutdown state.
#[derive(Clone)]
pub struct ShutdownToken {
  inner: Arc<AtomicBool>,
}

impl ShutdownToken {
  /// Creates a new shutdown token.
  ///
  /// Shutdown is not triggered in the initial state.
  ///
  /// # Returns
  /// New shutdown token
  pub fn new() -> Self {
    Self {
      inner: Arc::new(AtomicBool::new(false)),
    }
  }

  /// Triggers shutdown.
  ///
  /// This operation can be safely called from multiple threads.
  /// Once triggered, the state cannot be reset.
  pub fn trigger(&self) {
    self.inner.store(true, Ordering::SeqCst);
  }

  /// Checks whether shutdown has been triggered.
  ///
  /// # Returns
  /// `true` if shutdown has been triggered, `false` otherwise
  pub fn is_triggered(&self) -> bool {
    self.inner.load(Ordering::SeqCst)
  }
}

impl Default for ShutdownToken {
  fn default() -> Self {
    Self::new()
  }
}
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
