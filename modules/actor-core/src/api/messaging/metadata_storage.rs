use crate::api::{
  mailbox::ThreadSafe,
  messaging::{MessageMetadata, MetadataStorageMode, MetadataStorageRecord},
};

impl MetadataStorageMode for ThreadSafe {
  fn to_record(metadata: MessageMetadata<Self>) -> MetadataStorageRecord {
    MetadataStorageRecord::ThreadSafe(metadata)
  }

  fn from_record(record: MetadataStorageRecord) -> Option<MessageMetadata<Self>> {
    match record {
      | MetadataStorageRecord::ThreadSafe(metadata) => Some(metadata),
      | MetadataStorageRecord::SingleThread(_) => None,
    }
  }
}
