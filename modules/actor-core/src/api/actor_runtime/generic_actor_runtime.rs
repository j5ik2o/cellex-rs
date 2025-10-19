use cellex_utils_core_rs::{sync::ArcShared, Element};

use crate::{
  api::{
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    actor_scheduler::ActorSchedulerHandleBuilder,
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::AnyMessage,
    metrics::MetricsSinkShared,
    receive_timeout::{
      NoopReceiveTimeoutSchedulerFactoryProvider, ReceiveTimeoutSchedulerFactoryProviderShared,
      ReceiveTimeoutSchedulerFactoryShared,
    },
    supervision::escalation::{FailureEventHandler, FailureEventListener},
  },
  internal::{mailbox::PriorityMailboxSpawnerHandle, runtime_state::GenericActorRuntimeState},
};

/// Helper alias mapping a runtime bundle back to its use
/// cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
pub(crate) type BundleMailbox<MF> = MailboxOf<GenericActorRuntime<MF>>;

/// Runtime bundle that decorates a use cellex_actor_core_rs::api::mailbox::MailboxRuntime; with
/// ActorSystem-specific capabilities.
#[derive(Clone)]
pub struct GenericActorRuntime<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  core: GenericActorRuntimeState<MF>,
  receive_timeout_scheduler_factory_shared_opt:
    Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, BundleMailbox<MF>>>,
  receive_timeout_scheduler_factory_provider_shared_opt:
    Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<MF>>>,
  root_event_listener_opt: Option<FailureEventListener>,
  root_escalation_handler_opt: Option<FailureEventHandler>,
  metrics_sink_opt: Option<MetricsSinkShared>,
}

impl<MF> GenericActorRuntime<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  /// Creates a new bundle for the supplied use cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
  #[must_use]
  pub fn new(actor_runtime: MF) -> Self {
    Self {
      core: GenericActorRuntimeState::new(actor_runtime),
      receive_timeout_scheduler_factory_shared_opt: None,
      receive_timeout_scheduler_factory_provider_shared_opt: Some(ReceiveTimeoutSchedulerFactoryProviderShared::new(
        NoopReceiveTimeoutSchedulerFactoryProvider::default(),
      )),
      root_event_listener_opt: None,
      root_escalation_handler_opt: None,
      metrics_sink_opt: None,
    }
  }

  /// Returns a reference to the wrapped use cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
  #[must_use]
  pub fn mailbox_factory(&self) -> &MF {
    self.core.mailbox_factory()
  }

  /// Consumes the bundle and yields the underlying use
  /// cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
  #[must_use]
  pub fn into_mailbox_factory(self) -> MF {
    let Self { core, .. } = self;
    core.into_mailbox_factory()
  }

  /// Borrows the shared handle to the use cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
  #[must_use]
  pub fn mailbox_factory_shared(&self) -> ArcShared<MF> {
    self.core.mailbox_factory_shared()
  }

  /// Returns the configured mailbox-level receive-timeout factory, if any.
  #[must_use]
  pub fn receive_timeout_scheduler_factory_shared(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, BundleMailbox<MF>>> {
    self.receive_timeout_scheduler_factory_shared_opt.clone()
  }

  /// Returns the receive-timeout driver associated with the bundle.
  #[must_use]
  pub fn receive_timeout_scheduler_factory_provider_shared(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<MF>>> {
    self.receive_timeout_scheduler_factory_provider_shared_opt.clone()
  }

  /// Overrides the receive-timeout factory using a mailbox-level factory.
  #[must_use]
  pub fn with_receive_timeout_scheduler_factory_shared(
    mut self,
    factory: ReceiveTimeoutSchedulerFactoryShared<AnyMessage, BundleMailbox<MF>>,
  ) -> Self {
    self.receive_timeout_scheduler_factory_shared_opt = Some(factory);
    self
  }

  /// Sets the receive-timeout driver and returns the updated bundle.
  #[must_use]
  pub fn with_receive_timeout_driver(
    mut self,
    driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<MF>>>,
  ) -> Self {
    self.receive_timeout_scheduler_factory_provider_shared_opt = driver;
    self
  }

  /// Mutably replaces the receive-timeout driver.
  pub fn set_receive_timeout_scheduler_factory_provider_shared(
    &mut self,
    driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<BundleMailbox<MF>>>,
  ) {
    self.receive_timeout_scheduler_factory_provider_shared_opt = driver;
  }

  /// Builds a mailbox-level receive-timeout factory using the configured driver.
  #[must_use]
  pub fn receive_timeout_scheduler_factory_provider_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, BundleMailbox<MF>>> {
    self.receive_timeout_scheduler_factory_provider_shared_opt.as_ref().map(|driver| driver.build_factory())
  }

  /// Returns the configured root failure event listener.
  #[must_use]
  pub fn root_event_listener(&self) -> Option<FailureEventListener> {
    self.root_event_listener_opt.clone()
  }

  /// Overrides the root failure event listener.
  #[must_use]
  pub fn with_root_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.root_event_listener_opt = listener;
    self
  }

  /// Returns the configured root escalation handler.
  #[must_use]
  pub fn root_escalation_handler(&self) -> Option<FailureEventHandler> {
    self.root_escalation_handler_opt.clone()
  }

  /// Overrides the root escalation handler for the bundle.
  #[must_use]
  pub fn with_root_escalation_handler(mut self, handler: Option<FailureEventHandler>) -> Self {
    self.root_escalation_handler_opt = handler;
    self
  }

  /// Returns the metrics sink shared with spawned actors.
  #[must_use]
  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink_opt.clone()
  }

  /// Overrides the metrics sink with an optional handle.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink_opt = sink;
    self
  }

  /// Overrides the metrics sink using a concrete shared handle.
  #[must_use]
  pub fn with_metrics_sink_shared(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics_sink_opt = Some(sink);
    self
  }

  /// Builds a priority mailbox spawner scoped to the bundle configuration.
  #[must_use]
  pub fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, BundleMailbox<MF>>
  where
    M: Element,
    BundleMailbox<MF>: MailboxFactory,
    <BundleMailbox<MF> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
    <BundleMailbox<MF> as MailboxFactory>::Signal: Clone, {
    PriorityMailboxSpawnerHandle::new(self.mailbox_factory_shared()).with_metrics_sink(self.metrics_sink_opt.clone())
  }

  /// Overrides the scheduler builder with a concrete value.
  #[must_use]
  pub fn with_scheduler_builder(mut self, builder: ActorSchedulerHandleBuilder<MF>) -> Self {
    self.core.set_scheduler_builder(ArcShared::new(builder));
    self
  }

  /// Overrides the scheduler builder using a shared handle.
  #[must_use]
  pub fn with_scheduler_builder_shared(mut self, builder: ArcShared<ActorSchedulerHandleBuilder<MF>>) -> Self {
    self.core.set_scheduler_builder(builder);
    self
  }

  /// Returns the scheduler builder currently configured for the bundle.
  #[must_use]
  pub fn scheduler_builder(&self) -> ArcShared<ActorSchedulerHandleBuilder<MF>> {
    self.core.scheduler_builder()
  }
}

impl<MF> ActorRuntime for GenericActorRuntime<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  type MailboxFactory = MF;

  fn mailbox_factory(&self) -> &Self::MailboxFactory {
    GenericActorRuntime::mailbox_factory(self)
  }

  fn into_mailbox_factory(self) -> Self::MailboxFactory {
    GenericActorRuntime::into_mailbox_factory(self)
  }

  fn mailbox_factory_shared(&self) -> ArcShared<Self::MailboxFactory> {
    GenericActorRuntime::mailbox_factory_shared(self)
  }

  fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, Self::MailboxFactory>> {
    GenericActorRuntime::receive_timeout_scheduler_factory_shared(self)
  }

  fn receive_timeout_scheduler_factory_provider_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryProviderShared<Self::MailboxFactory>> {
    GenericActorRuntime::receive_timeout_scheduler_factory_provider_shared(self)
  }

  fn with_receive_timeout_scheduler_factory_shared(
    self,
    factory: ReceiveTimeoutSchedulerFactoryShared<AnyMessage, Self::MailboxFactory>,
  ) -> Self {
    GenericActorRuntime::with_receive_timeout_scheduler_factory_shared(self, factory)
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

  fn with_scheduler_builder(self, builder: ActorSchedulerHandleBuilder<Self::MailboxFactory>) -> Self {
    GenericActorRuntime::with_scheduler_builder(self, builder)
  }

  fn with_scheduler_builder_shared(
    self,
    builder: ArcShared<ActorSchedulerHandleBuilder<Self::MailboxFactory>>,
  ) -> Self {
    GenericActorRuntime::with_scheduler_builder_shared(self, builder)
  }

  fn scheduler_builder_shared(&self) -> ArcShared<ActorSchedulerHandleBuilder<Self::MailboxFactory>> {
    GenericActorRuntime::scheduler_builder(self)
  }
}
