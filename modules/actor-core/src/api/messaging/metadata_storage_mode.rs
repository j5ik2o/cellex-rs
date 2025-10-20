use crate::api::{
  mailbox::MailboxConcurrency,
  messaging::{metadata_storage_record::MetadataStorageRecord, MessageMetadata},
};

/// Marker trait describing how message metadata is persisted for a given mailbox concurrency mode.
pub trait MetadataStorageMode: MailboxConcurrency {
  /// Converts typed message metadata into the storage-agnostic record representation used by the runtime.
  #[allow(clippy::wrong_self_convention)]
  fn to_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord;

  /// Reconstructs typed message metadata from the storage record produced by [`MetadataStorageMode::to_record`].
  /// Returns `None` when the record does not contain metadata for this storage mode.
  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>>;
}
