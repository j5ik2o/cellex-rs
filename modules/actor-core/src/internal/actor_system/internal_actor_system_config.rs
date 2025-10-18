use crate::api::{
  actor_runtime::{ActorRuntime, MailboxOf},
  extensions::Extensions,
  failure_telemetry::FailureTelemetryShared,
  mailbox::{MailboxFactory, PriorityEnvelope},
  messaging::AnyMessage,
  metrics::MetricsSinkShared,
  process::pid::{NodeId, SystemId},
  receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
  supervision::{
    escalation::{FailureEventHandler, FailureEventListener},
    telemetry::{default_failure_telemetry_shared, TelemetryObservationConfig},
  },
};

/// Internal configuration used while assembling [`InternalActorSystem`].
pub struct InternalActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone,
  MailboxOf<AR>: MailboxFactory + Clone,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone, {
  /// Listener invoked for failures reaching the root guardian.
  pub(crate) root_event_listener:     Option<FailureEventListener>,
  /// Escalation handler invoked when failures bubble to the root guardian.
  pub(crate) root_escalation_handler: Option<FailureEventHandler>,
  /// Receive-timeout scheduler factory applied to newly spawned actors.
  pub(crate) receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<AR>>>,
  /// Metrics sink shared across the actor runtime.
  pub(crate) metrics_sink:            Option<MetricsSinkShared>,
  /// Shared registry of actor system extensions.
  pub(crate) extensions:              Extensions,
  /// Telemetry invoked when failures reach the root guardian。
  pub(crate) root_failure_telemetry:  FailureTelemetryShared,
  /// Observation config applied to telemetry calls.
  pub(crate) root_observation_config: TelemetryObservationConfig,
  /// Identifier assigned to the actor system for PID construction.
  pub(crate) system_id:               SystemId,
  /// Optional node identifier associated with this actor system instance.
  pub(crate) node_id:                 Option<NodeId>,
}

impl<AR> Default for InternalActorSystemConfig<AR>
where
  AR: ActorRuntime + Clone,
  MailboxOf<AR>: MailboxFactory + Clone,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
{
  fn default() -> Self {
    Self {
      root_event_listener:     None,
      root_escalation_handler: None,
      receive_timeout_factory: None,
      metrics_sink:            None,
      extensions:              Extensions::new(),
      root_failure_telemetry:  default_failure_telemetry_shared(),
      root_observation_config: TelemetryObservationConfig::new(),
      system_id:               SystemId::new("cellex"),
      node_id:                 None,
    }
  }
}
