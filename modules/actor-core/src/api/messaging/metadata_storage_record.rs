use crate::{MessageMetadata, SingleThread, ThreadSafe};

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
