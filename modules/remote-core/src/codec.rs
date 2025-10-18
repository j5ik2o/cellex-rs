use cellex_actor_core_rs::api::{
  mailbox::{PriorityChannel, SystemMessage, ThreadSafe},
  messaging::{MessageEnvelope, MessageMetadata},
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
  pub fn new(priority: i8, channel: PriorityChannel, payload: RemotePayloadFrame) -> Self {
    Self { priority, channel, payload }
  }
}

/// Encodes a [`RemoteEnvelope`] carrying serialized user messages or system messages into a
/// transport frame.
pub fn frame_from_serialized_envelope(
  envelope: RemoteEnvelope<MessageEnvelope<SerializedMessage>>,
) -> Result<RemoteMessageFrame, RemoteCodecError> {
  let (message_envelope, priority, channel) = envelope.into_parts_with_channel();
  match message_envelope {
    | MessageEnvelope::System(system) => {
      Ok(RemoteMessageFrame::new(priority, channel, RemotePayloadFrame::System(system)))
    },
    | MessageEnvelope::User(user) => {
      let (serialized, metadata) = user.into_parts::<ThreadSafe>();
      if metadata.is_some() {
        return Err(RemoteCodecError::UnsupportedMetadata);
      }
      Ok(RemoteMessageFrame::new(priority, channel, RemotePayloadFrame::User { serialized }))
    },
  }
}

/// Decodes a transport frame back into a [`RemoteEnvelope`] with serialized payloads.
#[must_use]
pub fn envelope_from_frame(frame: RemoteMessageFrame) -> RemoteEnvelope<MessageEnvelope<SerializedMessage>> {
  let RemoteMessageFrame { priority, channel, payload } = frame;
  let message_envelope = match payload {
    | RemotePayloadFrame::System(system) => MessageEnvelope::System(system),
    | RemotePayloadFrame::User { serialized } => MessageEnvelope::user(serialized),
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
  let envelope = user_envelope(serialized);
  RemoteEnvelope::new(envelope, priority, PriorityChannel::Control)
}
