use crate::{discard_metadata, store_metadata, MessageMetadata, MetadataKey, MetadataStorageMode};
use core::mem::{forget, ManuallyDrop};

/// Wrapper that holds a user message and metadata.
#[derive(Debug, Clone)]
pub struct UserMessage<U> {
  message: ManuallyDrop<U>,
  metadata_key: Option<MetadataKey>,
}

impl<U> UserMessage<U> {
  /// Creates a new `UserMessage` with only the message.
  ///
  /// # Arguments
  /// * `message` - User message
  pub fn new(message: U) -> Self {
    Self {
      message: ManuallyDrop::new(message),
      metadata_key: None,
    }
  }

  /// Creates a new `UserMessage` with message and metadata.
  ///
  /// If metadata is empty, it is created without metadata.
  ///
  /// # Arguments
  /// * `message` - User message
  /// * `metadata` - Message metadata
  pub fn with_metadata<C>(message: U, metadata: MessageMetadata<C>) -> Self
  where
    C: MetadataStorageMode, {
    if metadata.is_empty() {
      Self::new(message)
    } else {
      let key = store_metadata(metadata);
      Self {
        message: ManuallyDrop::new(message),
        metadata_key: Some(key),
      }
    }
  }

  /// Gets a reference to the message.
  ///
  /// # Returns
  /// Reference to the user message
  pub fn message(&self) -> &U {
    &*self.message
  }

  /// Gets the metadata key.
  ///
  /// # Returns
  /// `Some(MetadataKey)` if metadata exists, `None` otherwise
  pub fn metadata_key(&self) -> Option<MetadataKey> {
    self.metadata_key
  }

  /// Decomposes into message and metadata key.
  ///
  /// # Returns
  /// Tuple of `(message, metadata key)`
  pub fn into_parts(mut self) -> (U, Option<MetadataKey>) {
    let key = self.metadata_key.take();
    let message = unsafe { ManuallyDrop::take(&mut self.message) };
    forget(self);
    (message, key)
  }
}

impl<U> From<U> for UserMessage<U> {
  fn from(message: U) -> Self {
    Self::new(message)
  }
}

impl<U> Drop for UserMessage<U> {
  fn drop(&mut self) {
    if let Some(key) = self.metadata_key.take() {
      discard_metadata(key);
    }
    unsafe {
      ManuallyDrop::drop(&mut self.message);
    }
  }
}
