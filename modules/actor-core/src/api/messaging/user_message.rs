use core::mem::{forget, ManuallyDrop};

use crate::api::messaging::{metadata_storage_record::MetadataStorageRecord, MessageMetadata, MetadataStorageMode};

/// Wrapper that holds a user message and metadata.
#[derive(Debug, Clone)]
pub struct UserMessage<U> {
  message:  ManuallyDrop<U>,
  metadata: Option<MetadataStorageRecord>,
}

impl<U> UserMessage<U> {
  /// Creates a new `UserMessage` with only the message.
  ///
  /// # Arguments
  /// * `message` - User message
  pub const fn new(message: U) -> Self {
    Self { message: ManuallyDrop::new(message), metadata: None }
  }

  /// Creates a new `UserMessage` with message and metadata.
  ///
  /// If metadata is empty, it is created without metadata.
  ///
  /// # Arguments
  /// * `message` - User message
  /// * `metadata` - Message metadata
  pub fn with_metadata<Mode>(message: U, metadata: MessageMetadata<Mode>) -> Self
  where
    Mode: MetadataStorageMode, {
    if metadata.is_empty() {
      Self::new(message)
    } else {
      let record = Mode::to_record(metadata);
      Self { message: ManuallyDrop::new(message), metadata: Some(record) }
    }
  }

  /// Gets a reference to the message.
  ///
  /// # Returns
  /// Reference to the user message
  pub fn message(&self) -> &U {
    &self.message
  }

  /// Decomposes into message and metadata key.
  ///
  /// # Returns
  /// Tuple of `(message, metadata)`
  pub fn into_parts<Mode>(mut self) -> (U, Option<MessageMetadata<Mode>>)
  where
    Mode: MetadataStorageMode, {
    let record = self.metadata.take();
    let message = unsafe { ManuallyDrop::take(&mut self.message) };
    forget(self);
    let metadata = record.and_then(Mode::from_record);
    (message, metadata)
  }
}

impl<U> From<U> for UserMessage<U> {
  fn from(message: U) -> Self {
    Self::new(message)
  }
}

impl<U> Drop for UserMessage<U> {
  fn drop(&mut self) {
    unsafe {
      ManuallyDrop::drop(&mut self.message);
    }
  }
}
