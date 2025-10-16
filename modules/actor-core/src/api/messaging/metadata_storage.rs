use crate::api::mailbox::{MailboxConcurrency, SingleThread, ThreadSafe};
use crate::api::messaging::message_metadata::MessageMetadata;

/// Internal storage record used by metadata tables to preserve per-concurrency metadata.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum MetadataStorageRecord {
  ThreadSafe(MessageMetadata<ThreadSafe>),
  SingleThread(MessageMetadata<SingleThread>),
}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Send for MetadataStorageRecord {}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Sync for MetadataStorageRecord {}

/// Marker trait describing how message metadata is persisted for a given mailbox concurrency mode.
pub trait MetadataStorageMode: MailboxConcurrency {
  #[doc(hidden)]
  fn into_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord;

  #[doc(hidden)]
  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>>;
}

impl MetadataStorageMode for ThreadSafe {
  fn into_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord {
    MetadataStorageRecord::ThreadSafe(metadata)
  }

  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>> {
    match record {
      MetadataStorageRecord::ThreadSafe(metadata) => Some(metadata),
      MetadataStorageRecord::SingleThread(_) => None,
    }
  }
}

impl MetadataStorageMode for SingleThread {
  fn into_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord {
    MetadataStorageRecord::SingleThread(metadata)
  }

  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>> {
    match record {
      MetadataStorageRecord::SingleThread(metadata) => Some(metadata),
      MetadataStorageRecord::ThreadSafe(_) => None,
    }
  }
}
