use cellex_actor_core_rs::api::{mailbox::messages::PriorityChannel, process::pid::Pid};

use super::remote_payload_frame::RemotePayloadFrame;

/// Transport frame generated from a [`crate::remote_envelope::RemoteEnvelope`] ready for
/// serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteMessageFrame {
  /// Priority assigned to the message.
  pub priority: i8,
  /// Channel classification (regular or control).
  pub channel:  PriorityChannel,
  /// Encoded payload.
  pub payload:  RemotePayloadFrame,
  /// Reply-to PID when present.
  pub reply_to: Option<Pid>,
}

impl RemoteMessageFrame {
  /// Creates a new frame.
  #[must_use]
  pub const fn new(priority: i8, channel: PriorityChannel, payload: RemotePayloadFrame, reply_to: Option<Pid>) -> Self {
    Self { priority, channel, payload, reply_to }
  }
}
