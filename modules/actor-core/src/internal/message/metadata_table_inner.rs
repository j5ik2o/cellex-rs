use alloc::vec::Vec;

use super::metadata_table::MetadataKey;
use crate::api::messaging::{MessageMetadata, MetadataStorageMode, MetadataStorageRecord};

pub(crate) struct MetadataTableInner {
  pub(crate) entries:   Vec<Option<MetadataStorageRecord>>,
  pub(crate) free_list: Vec<MetadataKey>,
}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Send for MetadataTableInner {}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Sync for MetadataTableInner {}

impl MetadataTableInner {
  pub(crate) const fn new() -> Self {
    Self { entries: Vec::new(), free_list: Vec::new() }
  }

  pub(crate) fn store<C>(&mut self, metadata: MessageMetadata<C>) -> MetadataKey
  where
    C: MetadataStorageMode, {
    let stored = C::into_record(metadata);
    if let Some(key) = self.free_list.pop() {
      self.entries[key as usize] = Some(stored);
      key
    } else {
      let key = self.entries.len() as MetadataKey;
      self.entries.push(Some(stored));
      key
    }
  }

  pub(crate) fn discard(&mut self, key: MetadataKey) -> Option<MetadataStorageRecord> {
    let index = key as usize;
    if index >= self.entries.len() {
      return None;
    }
    let entry = self.entries[index].take();
    if entry.is_some() {
      self.free_list.push(key);
    }
    entry
  }

  pub(crate) fn take<C>(&mut self, key: MetadataKey) -> Option<MessageMetadata<C>>
  where
    C: MetadataStorageMode, {
    self.discard(key).and_then(C::from_record)
  }
}
