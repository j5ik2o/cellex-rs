use crate::api::mailbox::mailbox_runtime::MailboxRuntime;
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::supervision::failure::FailureInfo;
use crate::shared::failure_telemetry::FailureEventHandlerShared;
use crate::shared::failure_telemetry::FailureEventListenerShared;
use cellex_utils_core_rs::Element;

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
