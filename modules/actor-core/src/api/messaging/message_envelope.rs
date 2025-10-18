use cellex_utils_core_rs::Element;

use crate::api::{
  mailbox::{PriorityChannel, PriorityEnvelope, SystemMessage},
  messaging::{MessageMetadata, MetadataStorageMode, UserMessage},
};

/// Typed envelope that integrates user messages and system messages.
#[derive(Debug, Clone)]
pub enum MessageEnvelope<U> {
  /// Variant that holds a user message.
  User(UserMessage<U>),
  /// Variant that holds a system message.
  System(SystemMessage),
}

impl<U> MessageEnvelope<U>
where
  U: Element,
{
  /// Creates an envelope for a user message.
  ///
  /// # Arguments
  /// * `message` - User message
  pub fn user(message: U) -> Self {
    MessageEnvelope::User(UserMessage::new(message))
  }

  /// Creates an envelope for a user message with metadata.
  ///
  /// # Arguments
  /// * `message` - User message
  /// * `metadata` - Message metadata
  pub fn user_with_metadata<C>(message: U, metadata: MessageMetadata<C>) -> Self
  where
    C: MetadataStorageMode, {
    MessageEnvelope::User(UserMessage::with_metadata(message, metadata))
  }

  /// Wraps this envelope into a priority envelope with the specified priority.
  #[must_use]
  pub fn into_priority_envelope(self, priority: i8) -> PriorityEnvelope<Self> {
    PriorityEnvelope::new(self, priority)
  }

  /// Wraps this envelope into a control-channel priority envelope with the specified priority.
  #[must_use]
  pub fn into_control_envelope(self, priority: i8) -> PriorityEnvelope<Self> {
    PriorityEnvelope::with_channel(self, priority, PriorityChannel::Control)
  }

  /// Creates a control-channel envelope for a user message with the provided priority.
  #[must_use]
  pub fn control_user(message: U, priority: i8) -> PriorityEnvelope<Self> {
    MessageEnvelope::user(message).into_control_envelope(priority)
  }
}

#[cfg(test)]
mod tests {
  use alloc::string::String;

  use super::MessageEnvelope;
  use crate::api::mailbox::{PriorityChannel, SystemMessage};

  #[test]
  fn into_priority_envelope_retains_priority() {
    let message = MessageEnvelope::user(String::from("ping"));
    let envelope = message.into_priority_envelope(7);
    assert_eq!(envelope.priority(), 7);
    assert_eq!(envelope.channel(), PriorityChannel::Regular);
  }

  #[test]
  fn into_control_envelope_sets_control_channel() {
    let message: MessageEnvelope<SystemMessage> = MessageEnvelope::System(SystemMessage::Stop);
    let envelope = message.into_control_envelope(SystemMessage::Stop.priority());
    assert_eq!(envelope.channel(), PriorityChannel::Control);
    assert_eq!(envelope.priority(), SystemMessage::Stop.priority());
  }

  #[test]
  fn control_user_creates_control_priority_envelope() {
    let envelope = MessageEnvelope::control_user(String::from("urgent"), 10);
    assert_eq!(envelope.priority(), 10);
    assert_eq!(envelope.channel(), PriorityChannel::Control);
    let (message, _, _) = envelope.into_parts_with_channel();
    match message {
      | MessageEnvelope::User(user) => {
        let (value, metadata) = user.into_parts::<crate::api::mailbox::ThreadSafe>();
        assert_eq!(value, "urgent");
        assert!(metadata.is_none());
      },
      | MessageEnvelope::System(_) => panic!("expected user envelope"),
    }
  }
}
