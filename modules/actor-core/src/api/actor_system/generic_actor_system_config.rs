use core::num::NonZeroUsize;

use cellex_utils_core_rs::ArcShared;

use super::ActorSystemConfig as ActorSystemConfigTrait;
use crate::{
  api::{
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    extensions::{Extension, Extensions},
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{FailureTelemetryBuilderShared, FailureTelemetryObservationConfig, FailureTelemetryShared},
    },
    metrics::MetricsSinkShared,
    process::pid::{NodeId, SystemId},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Configuration options applied when constructing a
/// [`GenericActorSystem`](crate::api::actor_system::GenericActorSystem).
pub struct GenericActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  /// Listener invoked when failures bubble up to the root guardian.
  failure_event_listener_opt: Option<FailureEventListener>,
  /// Receive-timeout scheduler factory used by all actors spawned in the system.
  receive_timeout_scheduler_factory_shared_opt: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  /// Metrics sink shared across the actor runtime.
  metrics_sink_shared_opt: Option<MetricsSinkShared>,
  /// Telemetry invoked when failures reach the root guardian.
  failure_telemetry_shared_opt: Option<FailureTelemetryShared>,
  /// Builder used to create telemetry implementations.
  failure_telemetry_builder_shared_opt: Option<FailureTelemetryBuilderShared>,
  /// Observation configuration applied to telemetry calls.
  failure_observation_config_opt: Option<FailureTelemetryObservationConfig>,
  /// Extension registry configured for the actor system.
  extensions: Extensions,
  /// Default ReadyQueue worker count supplied by configuration.
  ready_queue_worker_count_opt: Option<NonZeroUsize>,
  /// Identifier assigned to the actor system for PID construction.
  system_id: SystemId,
  /// Optional node identifier associated with the actor system instance.
  node_id_opt: Option<NodeId>,
}

impl<AR> Default for GenericActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
{
  fn default() -> Self {
    Self {
      failure_event_listener_opt: None,
      receive_timeout_scheduler_factory_shared_opt: None,
      metrics_sink_shared_opt: None,
      failure_telemetry_shared_opt: None,
      failure_telemetry_builder_shared_opt: None,
      failure_observation_config_opt: None,
      extensions: Extensions::new(),
      ready_queue_worker_count_opt: None,
      system_id: SystemId::new("cellex"),
      node_id_opt: None,
    }
  }
}

impl<AR> GenericActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
{
  /// Sets the failure event listener.
  #[must_use]
  pub fn with_failure_event_listener_opt(mut self, listener: Option<FailureEventListener>) -> Self {
    self.failure_event_listener_opt = listener;
    self
  }

  /// Sets the receive-timeout factory.
  #[must_use]
  pub fn with_receive_timeout_scheduler_factory_shared_opt(
    mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  ) -> Self {
    self.receive_timeout_scheduler_factory_shared_opt = factory;
    self
  }

  /// Sets the metrics sink.
  #[must_use]
  pub fn with_metrics_sink_shared_opt(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink_shared_opt = sink;
    self
  }

  /// Sets the failure telemetry implementation.
  #[must_use]
  pub fn with_failure_telemetry_shared_opt(mut self, telemetry: Option<FailureTelemetryShared>) -> Self {
    self.failure_telemetry_shared_opt = telemetry;
    self
  }

  /// Sets the failure telemetry builder implementation.
  #[must_use]
  pub fn with_failure_telemetry_builder_shared_opt(mut self, builder: Option<FailureTelemetryBuilderShared>) -> Self {
    self.failure_telemetry_builder_shared_opt = builder;
    self
  }

  /// Sets telemetry observation configuration.
  #[must_use]
  pub fn with_failure_observation_config_opt(mut self, config: Option<FailureTelemetryObservationConfig>) -> Self {
    self.failure_observation_config_opt = config;
    self
  }

  /// Sets the default ReadyQueue worker count.
  #[must_use]
  pub const fn with_ready_queue_worker_count_opt(mut self, worker_count: Option<NonZeroUsize>) -> Self {
    self.ready_queue_worker_count_opt = worker_count;
    self
  }

  /// Convenience helper to set the ReadyQueue worker count explicitly.
  #[must_use]
  pub fn with_ready_queue_worker_count(self, worker_count: NonZeroUsize) -> Self {
    self.with_ready_queue_worker_count_opt(Some(worker_count))
  }

  /// Sets the system identifier used for PID construction.
  #[must_use]
  pub fn with_system_id(mut self, system_id: SystemId) -> Self {
    self.system_id = system_id;
    self
  }

  /// Sets the node identifier associated with this actor system.
  #[must_use]
  pub fn with_node_id_opt(mut self, node_id: Option<NodeId>) -> Self {
    self.node_id_opt = node_id;
    self
  }

  /// Convenience helper to set a concrete node identifier.
  #[must_use]
  pub fn with_node_id(self, node_id: NodeId) -> Self {
    self.with_node_id_opt(Some(node_id))
  }

  /// Sets the metrics sink using a concrete shared handle.
  #[must_use]
  pub fn with_metrics_sink_shared(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics_sink_shared_opt = Some(sink);
    self
  }

  /// Mutable setter for the failure event listener.
  pub fn set_failure_event_listener_opt(&mut self, listener: Option<FailureEventListener>) {
    self.failure_event_listener_opt = listener;
  }

  /// Mutable setter for the receive-timeout factory.
  pub fn set_receive_timeout_scheduler_factory_shared_opt(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  ) {
    self.receive_timeout_scheduler_factory_shared_opt = factory;
  }

  /// Mutable setter for the metrics sink.
  pub fn set_metrics_sink_shared_opt(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink_shared_opt = sink;
  }

  /// Mutable setter for the failure telemetry implementation.
  pub fn set_failure_telemetry_shared_opt(&mut self, telemetry: Option<FailureTelemetryShared>) {
    self.failure_telemetry_shared_opt = telemetry;
  }

  /// Mutable setter for the failure telemetry builder.
  pub fn set_failure_telemetry_builder_shared_opt(&mut self, builder: Option<FailureTelemetryBuilderShared>) {
    self.failure_telemetry_builder_shared_opt = builder;
  }

  /// Mutable setter for telemetry observation config.
  pub fn set_failure_observation_config_opt(&mut self, config: Option<FailureTelemetryObservationConfig>) {
    self.failure_observation_config_opt = config;
  }

  /// Mutable setter for the default ReadyQueue worker count.
  pub const fn set_ready_queue_worker_count_opt(&mut self, worker_count: Option<NonZeroUsize>) {
    self.ready_queue_worker_count_opt = worker_count;
  }

  /// Mutable setter for the ReadyQueue worker count.
  pub fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize) {
    self.ready_queue_worker_count_opt = Some(worker_count);
  }

  /// Mutable setter for the system identifier.
  pub fn set_system_id(&mut self, system_id: SystemId) {
    self.system_id = system_id;
  }

  /// Mutable setter for the node identifier.
  pub fn set_node_id_opt(&mut self, node_id: Option<NodeId>) {
    self.node_id_opt = node_id;
  }

  pub(crate) fn failure_event_listener_opt(&self) -> Option<FailureEventListener> {
    self.failure_event_listener_opt.clone()
  }

  pub(crate) fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>> {
    self.receive_timeout_scheduler_factory_shared_opt.clone()
  }

  pub(crate) fn metrics_sink_shared_opt(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink_shared_opt.clone()
  }

  pub(crate) fn failure_telemetry_shared_opt(&self) -> Option<FailureTelemetryShared> {
    self.failure_telemetry_shared_opt.clone()
  }

  pub(crate) fn failure_telemetry_builder_shared_opt(&self) -> Option<FailureTelemetryBuilderShared> {
    self.failure_telemetry_builder_shared_opt.clone()
  }

  pub(crate) fn failure_observation_config_opt(&self) -> Option<FailureTelemetryObservationConfig> {
    self.failure_observation_config_opt.clone()
  }

  pub(crate) const fn ready_queue_worker_count_opt(&self) -> Option<NonZeroUsize> {
    self.ready_queue_worker_count_opt
  }

  pub(crate) const fn system_id(&self) -> &SystemId {
    &self.system_id
  }

  pub(crate) fn node_id_opt(&self) -> Option<NodeId> {
    self.node_id_opt.clone()
  }

  /// Replaces the extension registry in the configuration.
  #[must_use]
  pub fn with_extensions(mut self, extensions: Extensions) -> Self {
    self.extensions = extensions;
    self
  }

  /// Registers an extension handle in the configuration.
  #[must_use]
  pub fn with_extension_handle<E>(self, extension: ArcShared<E>) -> Self
  where
    E: Extension + 'static, {
    let extensions = &self.extensions;
    extensions.register(extension);
    self
  }

  /// Registers an extension value in the configuration by wrapping it with `ArcShared`.
  #[must_use]
  pub fn with_extension_value<E>(self, extension: E) -> Self
  where
    E: Extension + 'static, {
    self.with_extension_handle(ArcShared::new(extension))
  }

  /// Returns the registered extensions.
  #[must_use]
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

impl<AR> ActorSystemConfigTrait<AR> for GenericActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
{
  fn with_failure_event_listener_opt(self, listener: Option<FailureEventListener>) -> Self {
    GenericActorSystemConfig::with_failure_event_listener_opt(self, listener)
  }

  fn with_receive_timeout_scheduler_factory_shared_opt(
    self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  ) -> Self {
    GenericActorSystemConfig::with_receive_timeout_scheduler_factory_shared_opt(self, factory)
  }

  fn with_metrics_sink_shared_opt(self, sink: Option<MetricsSinkShared>) -> Self {
    GenericActorSystemConfig::with_metrics_sink_shared_opt(self, sink)
  }

  fn with_failure_telemetry_shared_opt(self, telemetry: Option<FailureTelemetryShared>) -> Self {
    GenericActorSystemConfig::with_failure_telemetry_shared_opt(self, telemetry)
  }

  fn with_failure_telemetry_builder_shared_opt(self, builder: Option<FailureTelemetryBuilderShared>) -> Self {
    GenericActorSystemConfig::with_failure_telemetry_builder_shared_opt(self, builder)
  }

  fn with_failure_observation_config_opt(self, config: Option<FailureTelemetryObservationConfig>) -> Self {
    GenericActorSystemConfig::with_failure_observation_config_opt(self, config)
  }

  fn with_ready_queue_worker_count_opt(self, worker_count: Option<NonZeroUsize>) -> Self {
    GenericActorSystemConfig::with_ready_queue_worker_count_opt(self, worker_count)
  }

  fn with_ready_queue_worker_count(self, worker_count: NonZeroUsize) -> Self {
    GenericActorSystemConfig::with_ready_queue_worker_count(self, worker_count)
  }

  fn with_system_id(self, system_id: SystemId) -> Self {
    GenericActorSystemConfig::with_system_id(self, system_id)
  }

  fn with_node_id_opt(self, node_id: Option<NodeId>) -> Self {
    GenericActorSystemConfig::with_node_id_opt(self, node_id)
  }

  fn with_node_id(self, node_id: NodeId) -> Self {
    GenericActorSystemConfig::with_node_id(self, node_id)
  }

  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self {
    GenericActorSystemConfig::with_metrics_sink_shared(self, sink)
  }

  fn with_extensions(self, extensions: Extensions) -> Self {
    GenericActorSystemConfig::with_extensions(self, extensions)
  }

  fn with_extension_handle<E>(self, extension: ArcShared<E>) -> Self
  where
    E: Extension + 'static, {
    GenericActorSystemConfig::with_extension_handle(self, extension)
  }

  fn with_extension_value<E>(self, extension: E) -> Self
  where
    E: Extension + 'static, {
    GenericActorSystemConfig::with_extension_value(self, extension)
  }

  fn set_failure_event_listener_opt(&mut self, listener: Option<FailureEventListener>) {
    GenericActorSystemConfig::set_failure_event_listener_opt(self, listener);
  }

  fn set_receive_timeout_scheduler_factory_shared_opt(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  ) {
    GenericActorSystemConfig::set_receive_timeout_scheduler_factory_shared_opt(self, factory);
  }

  fn set_metrics_sink_shared_opt(&mut self, sink: Option<MetricsSinkShared>) {
    GenericActorSystemConfig::set_metrics_sink_shared_opt(self, sink);
  }

  fn set_failure_telemetry_shared_opt(&mut self, telemetry: Option<FailureTelemetryShared>) {
    GenericActorSystemConfig::set_failure_telemetry_shared_opt(self, telemetry);
  }

  fn set_failure_telemetry_builder_shared_opt(&mut self, builder: Option<FailureTelemetryBuilderShared>) {
    GenericActorSystemConfig::set_failure_telemetry_builder_shared_opt(self, builder);
  }

  fn set_failure_observation_config_opt(&mut self, config: Option<FailureTelemetryObservationConfig>) {
    GenericActorSystemConfig::set_failure_observation_config_opt(self, config);
  }

  fn set_ready_queue_worker_count_opt(&mut self, worker_count: Option<NonZeroUsize>) {
    GenericActorSystemConfig::set_ready_queue_worker_count_opt(self, worker_count);
  }

  fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize) {
    GenericActorSystemConfig::set_ready_queue_worker_count(self, worker_count);
  }

  fn set_system_id(&mut self, system_id: SystemId) {
    GenericActorSystemConfig::set_system_id(self, system_id);
  }

  fn set_node_id_opt(&mut self, node_id: Option<NodeId>) {
    GenericActorSystemConfig::set_node_id_opt(self, node_id);
  }

  fn failure_event_listener_opt(&self) -> Option<FailureEventListener> {
    GenericActorSystemConfig::failure_event_listener_opt(self)
  }

  fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>> {
    GenericActorSystemConfig::receive_timeout_scheduler_factory_shared_opt(self)
  }

  fn metrics_sink_shared_opt(&self) -> Option<MetricsSinkShared> {
    GenericActorSystemConfig::metrics_sink_shared_opt(self)
  }

  fn failure_telemetry_shared_opt(&self) -> Option<FailureTelemetryShared> {
    GenericActorSystemConfig::failure_telemetry_shared_opt(self)
  }

  fn failure_telemetry_builder_shared_opt(&self) -> Option<FailureTelemetryBuilderShared> {
    GenericActorSystemConfig::failure_telemetry_builder_shared_opt(self)
  }

  fn failure_observation_config_opt(&self) -> Option<FailureTelemetryObservationConfig> {
    GenericActorSystemConfig::failure_observation_config_opt(self)
  }

  fn ready_queue_worker_count_opt(&self) -> Option<NonZeroUsize> {
    GenericActorSystemConfig::ready_queue_worker_count_opt(self)
  }

  fn system_id(&self) -> &SystemId {
    GenericActorSystemConfig::system_id(self)
  }

  fn node_id_opt(&self) -> Option<NodeId> {
    GenericActorSystemConfig::node_id_opt(self)
  }

  fn extensions(&self) -> Extensions {
    GenericActorSystemConfig::extensions(self)
  }

  fn set_extensions(&mut self, extensions: Extensions) {
    GenericActorSystemConfig::set_extensions(self, extensions);
  }

  fn register_extension<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static, {
    GenericActorSystemConfig::register_extension(self, extension);
  }

  fn register_extension_dyn(&self, extension: ArcShared<dyn Extension>) {
    GenericActorSystemConfig::register_extension_dyn(self, extension);
  }
}
