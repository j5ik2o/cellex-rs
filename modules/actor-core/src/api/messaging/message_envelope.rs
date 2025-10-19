use cellex_utils_core_rs::Element;

use crate::api::{
  mailbox::messages::{PriorityChannel, PriorityEnvelope, SystemMessage},
  messaging::{MessageMetadata, MetadataStorageMode, UserMessage},
};

#[cfg(test)]
mod tests;

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
  pub const fn user(message: U) -> Self {
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
