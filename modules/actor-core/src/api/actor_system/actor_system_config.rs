use crate::api::actor_runtime::ActorRuntime;
use crate::api::actor_runtime::MailboxOf;
use crate::api::actor_runtime::MailboxQueueOf;
use crate::api::actor_runtime::MailboxSignalOf;
use crate::api::extensions::Extension;
use crate::api::extensions::Extensions;
use crate::api::failure_telemetry::FailureTelemetryBuilderShared;
use crate::api::failure_telemetry::FailureTelemetryShared;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::metrics::MetricsSinkShared;
use crate::api::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use cellex_utils_core_rs::ArcShared;
use core::num::NonZeroUsize;

/// Configuration options applied when constructing an [`ActorSystem`].
pub struct ActorSystemConfig<R>
where
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  /// Listener invoked when failures bubble up to the root guardian.
  failure_event_listener: Option<FailureEventListener>,
  /// Receive-timeout scheduler factory used by all actors spawned in the system.
  receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, MailboxOf<R>>>,
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
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
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
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
{
  /// Sets the failure event listener.
  pub fn with_failure_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.failure_event_listener = listener;
    self
  }

  /// Sets the receive-timeout factory.
  pub fn with_receive_timeout_factory(
    mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, MailboxOf<R>>>,
  ) -> Self {
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
  pub fn set_receive_timeout_factory(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, MailboxOf<R>>>,
  ) {
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

  pub(crate) fn receive_timeout_scheduler_factory_shared(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, MailboxOf<R>>> {
    self.receive_timeout_factory.clone()
  }

  pub(crate) fn metrics_sink_shared(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  pub(crate) fn failure_telemetry_shared(&self) -> Option<FailureTelemetryShared> {
    self.failure_telemetry.clone()
  }

  pub(crate) fn failure_telemetry_builder_shared(&self) -> Option<FailureTelemetryBuilderShared> {
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
