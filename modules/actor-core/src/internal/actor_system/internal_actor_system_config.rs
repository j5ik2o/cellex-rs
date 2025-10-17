use crate::api::actor_runtime::ActorRuntime;
use crate::api::actor_runtime::MailboxOf;
use crate::api::extensions::Extensions;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::telemetry::default_failure_telemetry;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::metrics::MetricsSinkShared;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use cellex_utils_core_rs::Element;

/// Internal configuration used while assembling [`InternalActorSystem`].
pub struct InternalActorSystemConfig<M, R>
where
  M: Element,
  R: ActorRuntime + Clone,
  MailboxOf<R>: MailboxFactory + Clone,
  <MailboxOf<R> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxFactory>::Signal: Clone, {
  /// Listener invoked for failures reaching the root guardian.
  pub(crate) root_event_listener: Option<FailureEventListener>,
  /// Escalation handler invoked when failures bubble to the root guardian.
  pub(crate) root_escalation_handler: Option<FailureEventHandler>,
  /// Receive-timeout scheduler factory applied to newly spawned actors.
  pub(crate) receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MailboxOf<R>>>,
  /// Metrics sink shared across the actor runtime.
  pub(crate) metrics_sink: Option<MetricsSinkShared>,
  /// Shared registry of actor system extensions.
  pub(crate) extensions: Extensions,
  /// Telemetry invoked when failures reach the root guardian。
  pub(crate) root_failure_telemetry: FailureTelemetryShared,
  /// Observation config applied to telemetry calls.
  pub(crate) root_observation_config: TelemetryObservationConfig,
}

impl<M, R> Default for InternalActorSystemConfig<M, R>
where
  M: Element,
  R: ActorRuntime + Clone,
  MailboxOf<R>: MailboxFactory + Clone,
  <MailboxOf<R> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxFactory>::Signal: Clone,
{
  fn default() -> Self {
    Self {
      root_event_listener: None,
      root_escalation_handler: None,
      receive_timeout_factory: None,
      metrics_sink: None,
      extensions: Extensions::new(),
      root_failure_telemetry: default_failure_telemetry(),
      root_observation_config: TelemetryObservationConfig::new(),
    }
  }
}
