use crate::api::mailbox::mailbox_concurrency::MailboxConcurrency;
use crate::api::messaging::metadata_storage_record::MetadataStorageRecord;
use crate::api::messaging::MessageMetadata;

/// Marker trait describing how message metadata is persisted for a given mailbox concurrency mode.
pub trait MetadataStorageMode: MailboxConcurrency {
  #[doc(hidden)]
  fn into_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord;

  #[doc(hidden)]
  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>>;
}
