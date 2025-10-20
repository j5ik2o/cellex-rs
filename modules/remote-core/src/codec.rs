use cellex_actor_core_rs::api::{
  mailbox::{
    messages::{PriorityChannel, SystemMessage},
    ThreadSafe,
  },
  messaging::{MessageEnvelope, MessageMetadata},
  process::pid::Pid,
};
use cellex_serialization_core_rs::message::SerializedMessage;

use crate::remote_envelope::RemoteEnvelope;

/// Errors that can occur when encoding or decoding remote envelopes.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RemoteCodecError {
  /// User metadata is currently unsupported for remote transport.
  #[error("user metadata is not supported in remote transport yet")]
  UnsupportedMetadata,
}

/// Transport frame generated from a [`RemoteEnvelope`] ready for serialization.
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

/// Payload variants for remote transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemotePayloadFrame {
  /// System message payload retained as-is.
  System(SystemMessage),
  /// User message encoded via the serialization layer.
  User {
    /// Serialized representation produced by the configured serializer.
    serialized: SerializedMessage,
  },
}

impl RemoteMessageFrame {
  /// Creates a new frame.
  #[must_use]
  pub const fn new(priority: i8, channel: PriorityChannel, payload: RemotePayloadFrame, reply_to: Option<Pid>) -> Self {
    Self { priority, channel, payload, reply_to }
  }
}

/// Encodes a [`RemoteEnvelope`] carrying serialized user messages or system messages into a
/// transport frame.
///
/// # Errors
/// Returns [`RemoteCodecError::UnsupportedMetadata`] when the envelope carries unsupported
/// metadata.
pub fn frame_from_serialized_envelope(
  envelope: RemoteEnvelope<MessageEnvelope<SerializedMessage>>,
) -> Result<RemoteMessageFrame, RemoteCodecError> {
  let (message_envelope, priority, channel) = envelope.into_parts_with_channel();
  match message_envelope {
    | MessageEnvelope::System(system) => {
      Ok(RemoteMessageFrame::new(priority, channel, RemotePayloadFrame::System(system), None))
    },
    | MessageEnvelope::User(user) => {
      let (serialized, metadata) = user.into_parts::<ThreadSafe>();
      if let Some(ref metadata) = metadata {
        if metadata.sender_as::<SerializedMessage>().is_some() || metadata.responder_as::<SerializedMessage>().is_some()
        {
          return Err(RemoteCodecError::UnsupportedMetadata);
        }
      }
      let reply_to = metadata.and_then(|meta| meta.responder_pid().cloned());
      Ok(RemoteMessageFrame::new(priority, channel, RemotePayloadFrame::User { serialized }, reply_to))
    },
  }
}

/// Decodes a transport frame back into a [`RemoteEnvelope`] with serialized payloads.
#[must_use]
pub fn envelope_from_frame(frame: RemoteMessageFrame) -> RemoteEnvelope<MessageEnvelope<SerializedMessage>> {
  let RemoteMessageFrame { priority, channel, payload, reply_to } = frame;
  let message_envelope = match payload {
    | RemotePayloadFrame::System(system) => MessageEnvelope::System(system),
    | RemotePayloadFrame::User { serialized } => {
      let metadata = match reply_to {
        | Some(pid) => MessageMetadata::<ThreadSafe>::new().with_responder_pid(pid),
        | None => MessageMetadata::<ThreadSafe>::new(),
      };
      MessageEnvelope::user_with_metadata(serialized, metadata)
    },
  };
  RemoteEnvelope::new(message_envelope, priority, channel)
}

/// Helper to wrap a serialized user message into a [`MessageEnvelope`].
#[must_use]
pub fn user_envelope(serialized: SerializedMessage) -> MessageEnvelope<SerializedMessage> {
  MessageEnvelope::user_with_metadata(serialized, MessageMetadata::<ThreadSafe>::new())
}

/// Helper to create a control channel remote envelope for a serialized user message.
#[must_use]
pub fn control_remote_envelope(
  serialized: SerializedMessage,
  priority: i8,
) -> RemoteEnvelope<MessageEnvelope<SerializedMessage>> {
  control_remote_envelope_with_reply(serialized, priority, None)
}

/// Helper to create a control channel remote envelope for a serialized user message with optional
/// reply-to PID.
#[must_use]
pub fn control_remote_envelope_with_reply(
  serialized: SerializedMessage,
  priority: i8,
  reply_to: Option<Pid>,
) -> RemoteEnvelope<MessageEnvelope<SerializedMessage>> {
  let metadata = reply_to.map_or_else(MessageMetadata::<ThreadSafe>::new, |pid| {
    MessageMetadata::<ThreadSafe>::new().with_responder_pid(pid)
  });
  let envelope = MessageEnvelope::user_with_metadata(serialized, metadata);
  RemoteEnvelope::new(envelope, priority, PriorityChannel::Control)
}
