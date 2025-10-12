use core::convert::Infallible;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use super::root_context::RootContext;
use super::{ActorSystemHandles, ActorSystemParts, Spawn, Timer};
use crate::api::guardian::AlwaysRestart;
use crate::runtime::mailbox::traits::MailboxPair;
use crate::runtime::mailbox::{MailboxOptions, PriorityMailboxSpawnerHandle};
use crate::runtime::message::DynMessage;
use crate::runtime::metrics::MetricsSinkShared;
use crate::runtime::scheduler::receive_timeout::NoopReceiveTimeoutDriver;
use crate::runtime::scheduler::SchedulerBuilder;
use crate::runtime::system::{InternalActorSystem, InternalActorSystemSettings};
use crate::serializer_extension_id;
use crate::{
  Extension, ExtensionId, Extensions, FailureEventHandler, FailureEventListener, FailureEventStream, MailboxRuntime,
  PriorityEnvelope, SerializerRegistryExtension,
};
use crate::{ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared};
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, QueueError};

/// Primary instance of the actor system.
///
/// Responsible for actor spawning, management, and message dispatching.
#[deprecated(since = "0.1.0", note = "Use NewActorSystem with a NewActorRuntimeBundle instead")]
pub struct ActorSystem<U, R, Strat = AlwaysRestart>
where
  U: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, ActorRuntimeBundle<R>>, {
  inner: InternalActorSystem<DynMessage, ActorRuntimeBundle<R>, Strat>,
  shutdown: ShutdownToken,
  extensions: Extensions,
  _marker: PhantomData<U>,
}

#[derive(Clone)]
pub(crate) struct ActorRuntimeBundleCore<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  mailbox_factory: ArcShared<R>,
  scheduler_builder: ArcShared<SchedulerBuilder<DynMessage, ActorRuntimeBundle<R>>>,
}

impl<R> ActorRuntimeBundleCore<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  #[must_use]
  pub(crate) fn new(mailbox_factory: R) -> Self {
    let shared_factory = ArcShared::new(mailbox_factory);
    Self {
      mailbox_factory: shared_factory,
      scheduler_builder: ArcShared::new(SchedulerBuilder::<DynMessage, ActorRuntimeBundle<R>>::priority()),
    }
  }

  #[must_use]
  pub(crate) fn mailbox_factory(&self) -> &R {
    &self.mailbox_factory
  }

  #[must_use]
  pub(crate) fn mailbox_factory_shared(&self) -> ArcShared<R> {
    self.mailbox_factory.clone()
  }

  #[must_use]
  pub(crate) fn into_mailbox_factory(self) -> R {
    self
      .mailbox_factory
      .try_unwrap()
      .unwrap_or_else(|shared| (*shared).clone())
  }

  #[must_use]
  pub(crate) fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, ActorRuntimeBundle<R>>> {
    self.scheduler_builder.clone()
  }

  pub(crate) fn set_scheduler_builder(
    &mut self,
    builder: ArcShared<SchedulerBuilder<DynMessage, ActorRuntimeBundle<R>>>,
  ) {
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

  /// Returns the shared runtime instance used by this factory.
  #[must_use]
  #[allow(dead_code)]
  pub(crate) fn runtime_shared(&self) -> ArcShared<R> {
    self.factory.clone()
  }

  /// Returns the metrics sink applied to spawned mailboxes, if any.
  #[must_use]
  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
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
#[deprecated(since = "0.1.0", note = "Use a NewActorRuntimeBundle implementation instead")]
pub struct ActorRuntimeBundle<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  core: ActorRuntimeBundleCore<R>,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, ActorRuntimeBundle<R>>>,
  receive_timeout_driver: Option<ReceiveTimeoutDriverShared<R>>,
  root_event_listener: Option<FailureEventListener>,
  root_escalation_handler: Option<FailureEventHandler>,
  metrics_sink: Option<MetricsSinkShared>,
}

impl<R> ActorRuntimeBundle<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new runtime bundle with the provided mailbox factory.
  #[must_use]
  pub fn new(mailbox_factory: R) -> Self {
    Self {
      core: ActorRuntimeBundleCore::new(mailbox_factory),
      receive_timeout_factory: None,
      receive_timeout_driver: Some(ReceiveTimeoutDriverShared::new(NoopReceiveTimeoutDriver::default())),
      root_event_listener: None,
      root_escalation_handler: None,
      metrics_sink: None,
    }
  }

  /// Returns a shared reference to the mailbox factory.
  #[must_use]
  pub fn mailbox_factory(&self) -> &R {
    self.core.mailbox_factory()
  }

  /// Consumes the bundle and returns the mailbox factory.
  #[must_use]
  pub fn into_mailbox_factory(self) -> R {
    let Self { core, .. } = self;
    core.into_mailbox_factory()
  }

  /// Returns the shared mailbox factory handle.
  #[must_use]
  pub fn mailbox_factory_shared(&self) -> ArcShared<R> {
    self.core.mailbox_factory_shared()
  }

  /// Returns the receive-timeout factory configured for this bundle.
  #[must_use]
  pub fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, ActorRuntimeBundle<R>>> {
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
    factory: ReceiveTimeoutFactoryShared<DynMessage, ActorRuntimeBundle<R>>,
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
  pub fn receive_timeout_driver_factory(
    &self,
  ) -> Option<ReceiveTimeoutFactoryShared<DynMessage, ActorRuntimeBundle<R>>> {
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
  pub fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, ActorRuntimeBundle<R>>
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
  pub fn with_scheduler_builder(mut self, builder: SchedulerBuilder<DynMessage, ActorRuntimeBundle<R>>) -> Self {
    self.core.set_scheduler_builder(ArcShared::new(builder));
    self
  }

  /// Overrides the scheduler builder using a pre-wrapped shared handle.
  #[must_use]
  pub fn with_scheduler_builder_shared(
    mut self,
    builder: ArcShared<SchedulerBuilder<DynMessage, ActorRuntimeBundle<R>>>,
  ) -> Self {
    self.core.set_scheduler_builder(builder);
    self
  }

  /// Returns the scheduler builder configured for this runtime bundle.
  #[must_use]
  pub fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, ActorRuntimeBundle<R>>> {
    self.core.scheduler_builder()
  }
}

impl<R> MailboxRuntime for ActorRuntimeBundle<R>
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
    self.mailbox_factory().build_mailbox(options)
  }
}

/// Configuration options applied when constructing an [`ActorSystem`].
#[deprecated(since = "0.1.0", note = "Use NewActorSystem with bundle-specified configuration instead")]
pub struct ActorSystemConfig<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  /// Listener invoked when failures bubble up to the root guardian.
  failure_event_listener: Option<FailureEventListener>,
  /// Receive-timeout scheduler factory used by all actors spawned in the system.
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, R>>,
  /// Metrics sink shared across the actor runtime.
  metrics_sink: Option<MetricsSinkShared>,
  /// Extension registry configured for the actor system.
  extensions: Extensions,
}

impl<R> Default for ActorSystemConfig<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn default() -> Self {
    Self {
      failure_event_listener: None,
      receive_timeout_factory: None,
      metrics_sink: None,
      extensions: Extensions::new(),
    }
  }
}

impl<R> ActorSystemConfig<R>
where
  R: MailboxRuntime + Clone + 'static,
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

  pub(crate) fn failure_event_listener(&self) -> Option<FailureEventListener> {
    self.failure_event_listener.clone()
  }

  pub(crate) fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, R>> {
    self.receive_timeout_factory.clone()
  }

  pub(crate) fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  /// Replaces the extension registry in the configuration.
  pub fn with_extensions(mut self, extensions: Extensions) -> Self {
    self.extensions = extensions;
    self
  }

  /// Registers an extension handle in the configuration.
  pub fn with_extension_handle<E>(self, extension: ArcShared<E>) -> Self
  where
    E: Extension, {
    let extensions = &self.extensions;
    extensions.register(extension);
    self
  }

  /// Registers an extension value in the configuration by wrapping it with `ArcShared`.
  pub fn with_extension_value<E>(self, extension: E) -> Self
  where
    E: Extension, {
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
    E: Extension, {
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
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, ActorRuntimeBundle<R>>, {
  system: ActorSystem<U, R, Strat>,
  _marker: PhantomData<U>,
}

impl<U, R> ActorSystem<U, R>
where
  U: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new actor system with the specified mailbox factory.
  ///
  /// # Arguments
  /// * `mailbox_factory` - Factory that generates mailboxes
  pub fn new(mailbox_factory: R) -> Self {
    Self::new_with_runtime(ActorRuntimeBundle::new(mailbox_factory), ActorSystemConfig::default())
  }

  /// Creates a new actor system with an explicit configuration.
  pub fn new_with_config(mailbox_factory: R, config: ActorSystemConfig<R>) -> Self {
    Self::new_with_runtime(ActorRuntimeBundle::new(mailbox_factory), config)
  }

  /// Creates a new actor system with a runtime bundle and configuration.
  pub fn new_with_runtime(runtime: ActorRuntimeBundle<R>, config: ActorSystemConfig<R>) -> Self {
    let bundle_receive_timeout = runtime.receive_timeout_factory();
    let bundle_root_listener = runtime.root_event_listener();
    let bundle_root_handler = runtime.root_escalation_handler();
    let extensions_handle = config.extensions();
    if extensions_handle.get(serializer_extension_id()).is_none() {
      let extension = ArcShared::new(SerializerRegistryExtension::new());
      extensions_handle.register(extension);
    }
    let extensions = extensions_handle.clone();
    let receive_timeout_factory = config
      .receive_timeout_factory()
      .map(|factory| factory.for_runtime_bundle())
      .or(bundle_receive_timeout)
      .or_else(|| runtime.receive_timeout_driver_factory());
    let root_event_listener = config.failure_event_listener().or(bundle_root_listener);
    let metrics_sink = config.metrics_sink().or_else(|| runtime.metrics_sink());
    let settings = InternalActorSystemSettings {
      root_event_listener,
      root_escalation_handler: bundle_root_handler,
      receive_timeout_factory,
      metrics_sink,
      extensions: extensions.clone(),
    };
    let scheduler_builder = runtime.scheduler_builder();
    Self {
      inner: InternalActorSystem::new_with_settings_and_builder(runtime, scheduler_builder, settings),
      shutdown: ShutdownToken::default(),
      extensions,
      _marker: PhantomData,
    }
  }

  /// Constructs an actor system and handles from parts.
  ///
  /// # Arguments
  /// * `parts` - Actor system parts
  ///
  /// # Returns
  /// Tuple of `(ActorSystem, ActorSystemHandles)`
  pub fn from_parts<S, T, E>(parts: ActorSystemParts<R, S, T, E>) -> (Self, ActorSystemHandles<S, T, E>)
  where
    S: Spawn,
    T: Timer,
    E: FailureEventStream, {
    let (mailbox_factory, handles) = parts.split();
    let config = ActorSystemConfig::default().with_failure_event_listener(Some(handles.event_stream.listener()));
    (Self::new_with_config(mailbox_factory, config), handles)
  }
}

impl<U, R, Strat> ActorSystem<U, R, Strat>
where
  U: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, ActorRuntimeBundle<R>>,
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
    ActorSystemRunner {
      system: self,
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
    E: Extension,
    F: FnOnce(&E) -> T, {
    self.extensions.with::<E, _, _>(id, f)
  }

  /// Registers an extension handle with the running actor system.
  pub fn register_extension<E>(&self, extension: ArcShared<E>)
  where
    E: Extension, {
    self.extensions.register(extension);
  }

  /// Registers a dynamically typed extension handle with the running actor system.
  pub fn register_extension_dyn(&self, extension: ArcShared<dyn Extension>) {
    self.extensions.register_dyn(extension);
  }

  /// Registers an extension value by wrapping it with `ArcShared`.
  pub fn register_extension_value<E>(&self, extension: E)
  where
    E: Extension, {
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

  /// Executes message dispatching in blocking mode until the specified condition is met.
  ///
  /// This function is only available when the standard library is enabled.
  ///
  /// # Arguments
  /// * `should_continue` - Closure that determines continuation condition. Continues execution while it returns `true`
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  #[cfg(feature = "std")]
  pub fn blocking_dispatch_loop<F>(
    &mut self,
    should_continue: F,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    F: FnMut() -> bool, {
    self.inner.blocking_dispatch_loop(should_continue)
  }

  /// Executes message dispatching permanently in blocking mode.
  ///
  /// This function is only available when the standard library is enabled. Does not terminate normally.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  #[cfg(feature = "std")]
  pub fn blocking_dispatch_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.blocking_dispatch_forever()
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
}

impl<U, R, Strat> ActorSystemRunner<U, R, Strat>
where
  U: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, ActorRuntimeBundle<R>>,
{
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
