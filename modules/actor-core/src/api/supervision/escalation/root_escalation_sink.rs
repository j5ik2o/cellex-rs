use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::time::Instant;

use crate::{
  api::{
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{
        default_failure_telemetry_shared, FailureSnapshot, FailureTelemetryObservationConfig, FailureTelemetryShared,
      },
      FailureEvent, FailureInfo,
    },
    mailbox::MailboxFactory,
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::AnyMessage,
    supervision::{EscalationSink, FailureEventHandler},
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
  failure_event_handler_opt: Option<FailureEventHandler>,
  failure_event_listener_opt: Option<FailureEventListener>,
  failure_telemetry_shared: FailureTelemetryShared,
  failure_telemetry_observation_config: FailureTelemetryObservationConfig,
  _marker: PhantomData<MF>,
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
  #[must_use]
  pub fn new() -> Self {
    Self {
      failure_event_handler_opt: None,
      failure_event_listener_opt: None,
      failure_telemetry_shared: default_failure_telemetry_shared(),
      failure_telemetry_observation_config: FailureTelemetryObservationConfig::default(),
      _marker: PhantomData,
    }
  }

  /// Sets the failure event handler.
  ///
  /// # Arguments
  ///
  /// * `handler` - Failure event handler, or `None`
  pub fn set_failure_event_handler_opt(&mut self, handler: Option<FailureEventHandler>) {
    self.failure_event_handler_opt = handler;
  }

  /// Sets the failure event listener.
  ///
  /// # Arguments
  ///
  /// * `listener` - Failure event listener, or `None`
  pub fn set_failure_event_listener_opt(&mut self, listener: Option<FailureEventListener>) {
    self.failure_event_listener_opt = listener;
  }

  /// Returns the currently registered telemetry implementation.
  #[must_use]
  pub fn failure_telemetry_shared(&self) -> FailureTelemetryShared {
    self.failure_telemetry_shared.clone()
  }

  /// Sets the telemetry implementation.
  pub fn set_failure_telemetry_shared(&mut self, telemetry: FailureTelemetryShared) {
    self.failure_telemetry_shared = telemetry;
  }

  /// Returns the telemetry observation config.
  #[must_use]
  pub const fn failure_telemetry_observation_config(&self) -> &FailureTelemetryObservationConfig {
    &self.failure_telemetry_observation_config
  }

  /// Sets the telemetry observation config.
  pub fn set_failure_telemetry_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
    self.failure_telemetry_observation_config = config;
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
    let start = if self.failure_telemetry_observation_config.should_record_timing()
      && self.failure_telemetry_observation_config.metrics_sink().is_some()
    {
      Some(Instant::now())
    } else {
      None
    };

    self.failure_telemetry_shared.with_ref(|telemetry| telemetry.on_failure(&snapshot));

    #[cfg(feature = "std")]
    let elapsed = start.map(|s| s.elapsed());
    #[cfg(not(feature = "std"))]
    let elapsed = None;

    self.failure_telemetry_observation_config.observe(elapsed);

    if let Some(handler) = self.failure_event_handler_opt.as_ref() {
      handler(&info);
    }

    if let Some(listener) = self.failure_event_listener_opt.as_ref() {
      listener(FailureEvent::RootEscalated(info));
    }

    Ok(())
  }
}
