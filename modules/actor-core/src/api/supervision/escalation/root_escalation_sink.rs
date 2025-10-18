use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::time::Instant;

use crate::api::{
  failure_telemetry::FailureTelemetryShared,
  mailbox::{MailboxFactory, PriorityEnvelope},
  messaging::AnyMessage,
  supervision::{
    escalation::escalation_sink::{EscalationSink, FailureEventHandler, FailureEventListener},
    failure::{FailureEvent, FailureInfo},
    telemetry::{default_failure_telemetry_shared, FailureSnapshot, TelemetryObservationConfig},
  },
};
/// `EscalationSink` implementation for root guardian.
///
/// Handles failures at the root level of the actor system.
/// Ultimately processes failures that cannot be escalated further.
pub struct RootEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  event_handler:  Option<FailureEventHandler>,
  event_listener: Option<FailureEventListener>,
  telemetry:      FailureTelemetryShared,
  observation:    TelemetryObservationConfig,
  _marker:        PhantomData<MF>,
}

impl<MF> RootEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  /// Creates a new `RootEscalationSink`.
  ///
  /// By default, no handler or listener is configured.
  pub fn new() -> Self {
    Self {
      event_handler:  None,
      event_listener: None,
      telemetry:      default_failure_telemetry_shared(),
      observation:    TelemetryObservationConfig::default(),
      _marker:        PhantomData,
    }
  }

  /// Sets the failure event handler.
  ///
  /// # Arguments
  ///
  /// * `handler` - Failure event handler, or `None`
  pub fn set_event_handler(&mut self, handler: Option<FailureEventHandler>) {
    self.event_handler = handler;
  }

  /// Sets the failure event listener.
  ///
  /// # Arguments
  ///
  /// * `listener` - Failure event listener, or `None`
  pub fn set_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.event_listener = listener;
  }

  /// Returns the currently registered telemetry implementation.
  pub fn telemetry(&self) -> FailureTelemetryShared {
    self.telemetry.clone()
  }

  /// Sets the telemetry implementation.
  pub fn set_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.telemetry = telemetry;
  }

  /// Returns the telemetry observation config.
  pub fn observation_config(&self) -> &TelemetryObservationConfig {
    &self.observation
  }

  /// Sets the telemetry observation config.
  pub fn set_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.observation = config;
  }
}

impl<MF> Default for RootEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<MF> EscalationSink<AnyMessage, MF> for RootEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  /// Processes failure information at root level.
  ///
  /// Performs log output, handler invocation, and listener notification.
  ///
  /// # Arguments
  ///
  /// * `info` - Failure information
  /// * `_already_handled` - Unused (always executes processing at root level)
  ///
  /// # Returns
  ///
  /// Always returns `Ok(())`
  fn handle(&mut self, info: FailureInfo, _already_handled: bool) -> Result<(), FailureInfo> {
    let snapshot = FailureSnapshot::from_failure_info(&info);
    #[cfg(feature = "std")]
    let start = if self.observation.should_record_timing() && self.observation.metrics_sink().is_some() {
      Some(Instant::now())
    } else {
      None
    };

    self.telemetry.with_ref(|telemetry| telemetry.on_failure(&snapshot));

    #[cfg(feature = "std")]
    let elapsed = start.map(|s| s.elapsed());
    #[cfg(not(feature = "std"))]
    let elapsed = None;

    self.observation.observe(elapsed);

    if let Some(handler) = self.event_handler.as_ref() {
      handler(&info);
    }

    if let Some(listener) = self.event_listener.as_ref() {
      listener(FailureEvent::RootEscalated(info.clone()));
    }

    Ok(())
  }
}
