use crate::api::actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::internal::mailbox::PriorityMailboxSpawnerHandle;
use crate::internal::metrics::MetricsSinkShared;
use crate::internal::runtime_state::GenericActorRuntimeState;
use crate::internal::scheduler::{NoopReceiveTimeoutSchedulerFactoryProvider, SchedulerBuilder};
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryProviderShared;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

/// Helper alias mapping a runtime bundle back to its mailbox runtime.
pub(crate) type BundleMailbox<R> = MailboxOf<GenericActorRuntime<R>>;

/// Runtime bundle that decorates a mailbox runtime with ActorSystem-specific capabilities.
#[derive(Clone)]
pub struct GenericActorRuntime<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  core: GenericActorRuntimeState<R>,
  receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, BundleMailbox<R>>>,
  receive_timeout_driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<R>>>,
  root_event_listener: Option<FailureEventListener>,
  root_escalation_handler: Option<FailureEventHandler>,
  metrics_sink: Option<MetricsSinkShared>,
}

impl<R> GenericActorRuntime<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new bundle for the supplied mailbox runtime.
  #[must_use]
  pub fn new(actor_runtime: R) -> Self {
    Self {
      core: GenericActorRuntimeState::new(actor_runtime),
      receive_timeout_factory: None,
      receive_timeout_driver: Some(ReceiveTimeoutSchedulerFactoryProviderShared::new(
        NoopReceiveTimeoutSchedulerFactoryProvider::default(),
      )),
      root_event_listener: None,
      root_escalation_handler: None,
      metrics_sink: None,
    }
  }

  /// Returns a reference to the wrapped mailbox runtime.
  #[must_use]
  pub fn mailbox_runtime(&self) -> &R {
    self.core.mailbox_runtime()
  }

  /// Consumes the bundle and yields the underlying mailbox runtime.
  #[must_use]
  pub fn into_mailbox_runtime(self) -> R {
    let Self { core, .. } = self;
    core.into_mailbox_runtime()
  }

  /// Borrows the shared handle to the mailbox runtime.
  #[must_use]
  pub fn mailbox_runtime_shared(&self) -> ArcShared<R> {
    self.core.mailbox_runtime_shared()
  }

  /// Returns the configured mailbox-level receive-timeout factory, if any.
  #[must_use]
  pub fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, BundleMailbox<R>>> {
    self.receive_timeout_factory.clone()
  }

  /// Returns the receive-timeout driver associated with the bundle.
  #[must_use]
  pub fn receive_timeout_driver(&self) -> Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<R>>> {
    self.receive_timeout_driver.clone()
  }

  /// Overrides the receive-timeout factory using a mailbox-level factory.
  #[must_use]
  pub fn with_receive_timeout_factory(
    mut self,
    factory: ReceiveTimeoutSchedulerFactoryShared<DynMessage, BundleMailbox<R>>,
  ) -> Self {
    self.receive_timeout_factory = Some(factory);
    self
  }

  /// Sets the receive-timeout driver and returns the updated bundle.
  #[must_use]
  pub fn with_receive_timeout_driver(
    mut self,
    driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<R>>>,
  ) -> Self {
    self.receive_timeout_driver = driver;
    self
  }

  /// Mutably replaces the receive-timeout driver.
  pub fn set_receive_timeout_driver(
    &mut self,
    driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<R>>>,
  ) {
    self.receive_timeout_driver = driver;
  }

  /// Builds a mailbox-level receive-timeout factory using the configured driver.
  #[must_use]
  pub fn receive_timeout_driver_factory(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, BundleMailbox<R>>> {
    self
      .receive_timeout_driver
      .as_ref()
      .map(|driver| driver.build_factory())
  }

  /// Returns the configured root failure event listener.
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

  /// Returns the configured root escalation handler.
  #[must_use]
  pub fn root_escalation_handler(&self) -> Option<FailureEventHandler> {
    self.root_escalation_handler.clone()
  }

  /// Overrides the root escalation handler for the bundle.
  #[must_use]
  pub fn with_root_escalation_handler(mut self, handler: Option<FailureEventHandler>) -> Self {
    self.root_escalation_handler = handler;
    self
  }

  /// Returns the metrics sink shared with spawned actors.
  #[must_use]
  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  /// Overrides the metrics sink with an optional handle.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Overrides the metrics sink using a concrete shared handle.
  #[must_use]
  pub fn with_metrics_sink_shared(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics_sink = Some(sink);
    self
  }

  /// Builds a priority mailbox spawner scoped to the bundle configuration.
  #[must_use]
  pub fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, BundleMailbox<R>>
  where
    M: Element,
    BundleMailbox<R>: MailboxFactory,
    <BundleMailbox<R> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
    <BundleMailbox<R> as MailboxFactory>::Signal: Clone, {
    PriorityMailboxSpawnerHandle::new(self.mailbox_runtime_shared()).with_metrics_sink(self.metrics_sink.clone())
  }

  /// Overrides the scheduler builder with a concrete value.
  #[must_use]
  pub fn with_scheduler_builder(mut self, builder: SchedulerBuilder<DynMessage, R>) -> Self {
    self.core.set_scheduler_builder(ArcShared::new(builder));
    self
  }

  /// Overrides the scheduler builder using a shared handle.
  #[must_use]
  pub fn with_scheduler_builder_shared(mut self, builder: ArcShared<SchedulerBuilder<DynMessage, R>>) -> Self {
    self.core.set_scheduler_builder(builder);
    self
  }

  /// Returns the scheduler builder currently configured for the bundle.
  #[must_use]
  pub fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, R>> {
    self.core.scheduler_builder()
  }
}

impl<R> ActorRuntime for GenericActorRuntime<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  type MailboxFactory = R;

  fn mailbox_factory(&self) -> &Self::MailboxFactory {
    GenericActorRuntime::mailbox_runtime(self)
  }

  fn into_mailbox_factory(self) -> Self::MailboxFactory {
    GenericActorRuntime::into_mailbox_runtime(self)
  }

  fn mailbox_factory_shared(&self) -> ArcShared<Self::MailboxFactory> {
    GenericActorRuntime::mailbox_runtime_shared(self)
  }

  fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, Self::MailboxFactory>> {
    GenericActorRuntime::receive_timeout_factory(self)
  }

  fn receive_timeout_scheduler_factory_provider_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryProviderShared<Self::MailboxFactory>> {
    GenericActorRuntime::receive_timeout_driver(self)
  }

  fn with_receive_timeout_scheduler_factory_shared(
    self,
    factory: ReceiveTimeoutSchedulerFactoryShared<DynMessage, Self::MailboxFactory>,
  ) -> Self {
    GenericActorRuntime::with_receive_timeout_factory(self, factory)
  }

  fn with_receive_timeout_scheduler_factory_provider_shared_opt(
    self,
    driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<Self::MailboxFactory>>,
  ) -> Self {
    GenericActorRuntime::with_receive_timeout_driver(self, driver)
  }

  fn root_event_listener_opt(&self) -> Option<FailureEventListener> {
    GenericActorRuntime::root_event_listener(self)
  }

  fn with_root_event_listener_opt(self, listener: Option<FailureEventListener>) -> Self {
    GenericActorRuntime::with_root_event_listener(self, listener)
  }

  fn root_escalation_handler_opt(&self) -> Option<FailureEventHandler> {
    GenericActorRuntime::root_escalation_handler(self)
  }

  fn with_root_escalation_handler_opt(self, handler: Option<FailureEventHandler>) -> Self {
    GenericActorRuntime::with_root_escalation_handler(self, handler)
  }

  fn metrics_sink_shared_opt(&self) -> Option<MetricsSinkShared> {
    GenericActorRuntime::metrics_sink(self)
  }

  fn with_metrics_sink_shared_opt(self, sink: Option<MetricsSinkShared>) -> Self {
    GenericActorRuntime::with_metrics_sink(self, sink)
  }

  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self {
    GenericActorRuntime::with_metrics_sink_shared(self, sink)
  }

  fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, Self::MailboxFactory>
  where
    M: Element,
    MailboxQueueOf<Self, PriorityEnvelope<M>>: Clone,
    MailboxSignalOf<Self>: Clone, {
    GenericActorRuntime::priority_mailbox_spawner(self)
  }

  fn with_scheduler_builder(self, builder: SchedulerBuilder<DynMessage, Self::MailboxFactory>) -> Self {
    GenericActorRuntime::with_scheduler_builder(self, builder)
  }

  fn with_scheduler_builder_shared(
    self,
    builder: ArcShared<SchedulerBuilder<DynMessage, Self::MailboxFactory>>,
  ) -> Self {
    GenericActorRuntime::with_scheduler_builder_shared(self, builder)
  }

  fn scheduler_builder_shared(&self) -> ArcShared<SchedulerBuilder<DynMessage, Self::MailboxFactory>> {
    GenericActorRuntime::scheduler_builder(self)
  }
}
