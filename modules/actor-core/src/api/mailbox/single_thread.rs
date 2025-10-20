use crate::api::{
  mailbox::mailbox_concurrency::MailboxConcurrency,
  messaging::{MessageMetadata, MetadataStorageMode, MetadataStorageRecord},
};

/// Single-threaded mailbox mode without additional synchronization requirements.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleThread;

impl MailboxConcurrency for SingleThread {}

impl MetadataStorageMode for SingleThread {
  fn to_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord {
    MetadataStorageRecord::SingleThread(metadata)
  }

  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>> {
    match record {
      | MetadataStorageRecord::SingleThread(metadata) => Some(metadata),
      | MetadataStorageRecord::ThreadSafe(_) => None,
    }
  }
}
