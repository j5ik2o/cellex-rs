use core::marker::PhantomData;

use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

use super::failure::FailureEvent;
use super::telemetry::{default_failure_telemetry, FailureTelemetry};
use crate::{FailureEventHandlerShared, FailureEventListenerShared};
use crate::{FailureInfo, MailboxRuntime, PriorityEnvelope};

/// Handler for notifying failure events externally.
///
/// Receives actor failure information and performs tasks like logging or notifications to monitoring systems.
pub type FailureEventHandler = FailureEventHandlerShared;

/// Listener for receiving failure events as a stream.
///
/// Subscribes to failure events from the entire actor system and executes custom processing.
pub type FailureEventListener = FailureEventListenerShared;

/// Sink for controlling how `FailureInfo` is propagated upward.
///
/// Defines escalation processing for actor failure information.
pub trait EscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Processes failure information.
  ///
  /// # Arguments
  ///
  /// * `info` - Failure information
  /// * `already_handled` - If `true`, indicates that processing has already been completed locally
  ///
  /// # Returns
  ///
  /// `Ok(())` on success, `Err(FailureInfo)` if processing failed
  fn handle(&mut self, info: FailureInfo, already_handled: bool) -> Result<(), FailureInfo>;
}

/// `EscalationSink` implementation for root guardian.
///
/// Handles failures at the root level of the actor system.
/// Ultimately processes failures that cannot be escalated further.
pub struct RootEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  event_handler: Option<FailureEventHandler>,
  event_listener: Option<FailureEventListener>,
  telemetry: ArcShared<dyn FailureTelemetry>,
  _marker: PhantomData<(M, R)>,
}

impl<M, R> RootEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new `RootEscalationSink`.
  ///
  /// By default, no handler or listener is configured.
  pub fn new() -> Self {
    Self {
      event_handler: None,
      event_listener: None,
      telemetry: default_failure_telemetry(),
      _marker: PhantomData,
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
  pub fn telemetry(&self) -> ArcShared<dyn FailureTelemetry> {
    self.telemetry.clone()
  }

  /// Sets the telemetry implementation.
  pub fn set_telemetry(&mut self, telemetry: ArcShared<dyn FailureTelemetry>) {
    self.telemetry = telemetry;
  }
}

impl<M, R> Default for RootEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<M, R> EscalationSink<M, R> for RootEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
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
    self.telemetry.on_failure(&info);

    if let Some(handler) = self.event_handler.as_ref() {
      handler(&info);
    }

    if let Some(listener) = self.event_listener.as_ref() {
      listener(FailureEvent::RootEscalated(info.clone()));
    }

    Ok(())
  }
}

#[cfg(all(test, feature = "std"))]
mod tests {
  use super::*;
  use crate::{ActorFailure, ActorId, ActorPath};
  use crate::runtime::mailbox::test_support::TestMailboxRuntime;
  use std::sync::{Arc, Mutex};

  #[derive(Clone, Default)]
  struct RecordingTelemetry {
    events: Arc<Mutex<Vec<FailureInfo>>>,
  }

  impl RecordingTelemetry {
    fn new() -> (Self, Arc<Mutex<Vec<FailureInfo>>>) {
      let events = Arc::new(Mutex::new(Vec::new()));
      (Self { events: events.clone() }, events)
    }
  }

  impl FailureTelemetry for RecordingTelemetry {
    fn on_failure(&self, info: &FailureInfo) {
      let mut guard = self.events.lock().unwrap();
      guard.push(info.clone());
    }
  }

  #[derive(Clone, Debug)]
  struct DummyMessage;

  #[test]
  fn root_escalation_sink_invokes_telemetry() {
    let (telemetry_impl, events) = RecordingTelemetry::new();
    let telemetry_shared = ArcShared::from_arc(Arc::new(telemetry_impl) as Arc<dyn FailureTelemetry>);

    let mut sink: RootEscalationSink<DummyMessage, TestMailboxRuntime> = RootEscalationSink::new();
    sink.set_telemetry(telemetry_shared);

    let failure = ActorFailure::from_message("boom");
    let info = FailureInfo::new(ActorId(1), ActorPath::new(), failure);

    sink.handle(info.clone(), false).expect("sink handle");

    let guard = events.lock().unwrap();
    assert_eq!(guard.len(), 1);
    assert_eq!(guard[0].actor, ActorId(1));
    assert_eq!(guard[0].description(), info.description());
  }

  #[test]
  fn telemetry_default_is_noop() {
    let mut sink: RootEscalationSink<DummyMessage, TestMailboxRuntime> = RootEscalationSink::new();
    let failure = ActorFailure::from_message("boom");
    let info = FailureInfo::new(ActorId(7), ActorPath::new(), failure);

    // Should not panic even though telemetry does nothing by default.
    sink.handle(info, false).expect("sink handle");
  }
}
