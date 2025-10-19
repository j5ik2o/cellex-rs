use core::fmt;

use cellex_utils_core_rs::QueueError;

use crate::api::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage};

/// Errors that can occur during `ask` processing.
#[derive(Debug)]
pub enum AskError {
  /// Responder not found
  MissingResponder,
  /// Message send failed
  SendFailed(QueueError<PriorityEnvelope<AnyMessage>>),
  /// Responder was dropped before responding
  ResponderDropped,
  /// Response await was cancelled
  ResponseAwaitCancelled,
  /// Timeout occurred
  Timeout,
}

impl fmt::Display for AskError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | AskError::MissingResponder => write!(f, "no responder available for current message"),
      | AskError::SendFailed(err) => write!(f, "failed to send ask response: {:?}", err),
      | AskError::ResponderDropped => write!(f, "ask responder dropped before sending a response"),
      | AskError::ResponseAwaitCancelled => write!(f, "ask future was cancelled before completion"),
      | AskError::Timeout => write!(f, "ask future timed out"),
    }
  }
}

impl From<QueueError<PriorityEnvelope<AnyMessage>>> for AskError {
  fn from(value: QueueError<PriorityEnvelope<AnyMessage>>) -> Self {
    AskError::SendFailed(value)
  }
}
