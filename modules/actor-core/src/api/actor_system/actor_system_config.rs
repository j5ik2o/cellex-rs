use core::num::NonZeroUsize;

use cellex_utils_core_rs::ArcShared;

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

/// Describes configuration primitives consumed by actor system builders.
pub trait ActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Self: Sized, {
  /// Applies a failure event listener override.
  fn with_failure_event_listener_opt(self, listener: Option<FailureEventListener>) -> Self;

  /// Applies a receive-timeout scheduler factory override.
  fn with_receive_timeout_scheduler_factory_shared_opt(
    self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  ) -> Self;

  /// Applies a metrics sink override.
  fn with_metrics_sink_shared_opt(self, sink: Option<MetricsSinkShared>) -> Self;

  /// Applies a failure telemetry override.
  fn with_failure_telemetry_shared_opt(self, telemetry: Option<FailureTelemetryShared>) -> Self;

  /// Applies a failure telemetry builder override.
  fn with_failure_telemetry_builder_shared_opt(self, builder: Option<FailureTelemetryBuilderShared>) -> Self;

  /// Applies telemetry observation configuration.
  fn with_failure_observation_config_opt(self, config: Option<FailureTelemetryObservationConfig>) -> Self;

  /// Applies an optional ReadyQueue worker count override.
  fn with_ready_queue_worker_count_opt(self, worker_count: Option<NonZeroUsize>) -> Self;

  /// Applies an explicit ReadyQueue worker count override.
  fn with_ready_queue_worker_count(self, worker_count: NonZeroUsize) -> Self;

  /// Applies a system identifier override.
  fn with_system_id(self, system_id: SystemId) -> Self;

  /// Applies an optional node identifier override.
  fn with_node_id_opt(self, node_id: Option<NodeId>) -> Self;

  /// Applies a concrete node identifier override.
  fn with_node_id(self, node_id: NodeId) -> Self;

  /// Applies a concrete metrics sink override.
  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self;

  /// Applies a fresh extensions registry.
  fn with_extensions(self, extensions: Extensions) -> Self;

  /// Registers an extension handle on the configuration.
  fn with_extension_handle<E>(self, extension: ArcShared<E>) -> Self
  where
    E: Extension + 'static;

  /// Registers an extension value on the configuration.
  fn with_extension_value<E>(self, extension: E) -> Self
  where
    E: Extension + 'static;

  /// Mutably sets the failure event listener.
  fn set_failure_event_listener_opt(&mut self, listener: Option<FailureEventListener>);

  /// Mutably sets the receive-timeout scheduler factory.
  fn set_receive_timeout_scheduler_factory_shared_opt(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  );

  /// Mutably sets the metrics sink.
  fn set_metrics_sink_shared_opt(&mut self, sink: Option<MetricsSinkShared>);

  /// Mutably sets the failure telemetry implementation.
  fn set_failure_telemetry_shared_opt(&mut self, telemetry: Option<FailureTelemetryShared>);

  /// Mutably sets the failure telemetry builder.
  fn set_failure_telemetry_builder_shared_opt(&mut self, builder: Option<FailureTelemetryBuilderShared>);

  /// Mutably sets the telemetry observation configuration.
  fn set_failure_observation_config_opt(&mut self, config: Option<FailureTelemetryObservationConfig>);

  /// Mutably sets the optional ReadyQueue worker count.
  fn set_ready_queue_worker_count_opt(&mut self, worker_count: Option<NonZeroUsize>);

  /// Mutably sets the ReadyQueue worker count.
  fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize);

  /// Mutably sets the system identifier.
  fn set_system_id(&mut self, system_id: SystemId);

  /// Mutably sets the optional node identifier.
  fn set_node_id_opt(&mut self, node_id: Option<NodeId>);

  /// Returns the failure event listener override.
  fn failure_event_listener_opt(&self) -> Option<FailureEventListener>;

  /// Returns the receive-timeout scheduler factory override.
  fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>;

  /// Returns the metrics sink override.
  fn metrics_sink_shared_opt(&self) -> Option<MetricsSinkShared>;

  /// Returns the failure telemetry override.
  fn failure_telemetry_shared_opt(&self) -> Option<FailureTelemetryShared>;

  /// Returns the failure telemetry builder override.
  fn failure_telemetry_builder_shared_opt(&self) -> Option<FailureTelemetryBuilderShared>;

  /// Returns the telemetry observation configuration override.
  fn failure_observation_config_opt(&self) -> Option<FailureTelemetryObservationConfig>;

  /// Returns the optional ReadyQueue worker count override.
  fn ready_queue_worker_count_opt(&self) -> Option<NonZeroUsize>;

  /// Returns the configured system identifier.
  fn system_id(&self) -> &SystemId;

  /// Returns the optional node identifier.
  fn node_id_opt(&self) -> Option<NodeId>;

  /// Returns the extensions registry.
  fn extensions(&self) -> Extensions;

  /// Mutably replaces the extensions registry.
  fn set_extensions(&mut self, extensions: Extensions);

  /// Registers an extension handle on the existing registry.
  fn register_extension<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static;

  /// Registers a trait-object extension on the existing registry.
  fn register_extension_dyn(&self, extension: ArcShared<dyn Extension>);
}
