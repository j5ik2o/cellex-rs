use cellex_utils_core_rs::Element;

use crate::{
  api::{
    failure::{failure_telemetry::FailureEventHandlerShared, FailureInfo},
    mailbox::MailboxFactory,
  },
  shared::mailbox::messages::PriorityEnvelope,
};

/// Handler for notifying failure events externally.
///
/// Receives actor failure information and performs tasks like logging or notifications to
/// monitoring systems.
pub type FailureEventHandler = FailureEventHandlerShared;

/// Sink for controlling how `FailureInfo` is propagated upward.
///
/// Defines escalation processing for actor failure information.
pub trait EscalationSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
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
  ///
  /// # Errors
  /// Returns [`FailureInfo`] when the sink rejects the escalation request and propagates it
  /// back to the caller for further handling.
  fn handle(&mut self, info: FailureInfo, already_handled: bool) -> Result<(), FailureInfo>;
}
