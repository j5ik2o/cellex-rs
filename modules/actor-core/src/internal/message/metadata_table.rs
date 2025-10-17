use crate::api::messaging::{MessageMetadata, MetadataStorageMode};

#[cfg(not(target_has_atomic = "ptr"))]
use core::cell::RefCell;
#[cfg(not(target_has_atomic = "ptr"))]
use critical_section::Mutex;
#[cfg(target_has_atomic = "ptr")]
use spin::{Mutex, Once};

use super::metadata_table_inner::MetadataTableInner;

/// Key type for referencing metadata.
pub type MetadataKey = u32;

#[cfg(target_has_atomic = "ptr")]
pub struct MetadataTable {
  inner: Mutex<MetadataTableInner>,
}

#[cfg(target_has_atomic = "ptr")]
impl MetadataTable {
  const fn new() -> Self {
    Self {
      inner: Mutex::new(MetadataTableInner::new()),
    }
  }

  pub fn store<C>(&self, metadata: MessageMetadata<C>) -> MetadataKey
  where
    C: MetadataStorageMode, {
    let mut guard = self.inner.lock();
    guard.store(metadata)
  }

  pub fn take<C>(&self, key: MetadataKey) -> Option<MessageMetadata<C>>
  where
    C: MetadataStorageMode, {
    let mut guard = self.inner.lock();
    guard.take(key)
  }

  pub fn discard(&self, key: MetadataKey) {
    let mut guard = self.inner.lock();
    guard.discard(key);
  }
}

#[cfg(target_has_atomic = "ptr")]
fn global_table() -> &'static MetadataTable {
  static TABLE: Once<MetadataTable> = Once::new();
  TABLE.call_once(MetadataTable::new)
}

#[cfg(not(target_has_atomic = "ptr"))]
pub struct MetadataTable {
  inner: Mutex<RefCell<MetadataTableInner>>,
}

#[cfg(not(target_has_atomic = "ptr"))]
impl MetadataTable {
  const fn new() -> Self {
    Self {
      inner: Mutex::new(RefCell::new(MetadataTableInner::new())),
    }
  }

  fn with_inner<R>(&self, f: impl FnOnce(&mut MetadataTableInner) -> R) -> R {
    critical_section::with(|cs| {
      let mut guard = self.inner.borrow(cs).borrow_mut();
      f(&mut guard)
    })
  }

  pub fn store<C>(&self, metadata: MessageMetadata<C>) -> MetadataKey
  where
    C: MetadataStorageMode, {
    self.with_inner(|inner| inner.store(metadata))
  }

  pub fn take<C>(&self, key: MetadataKey) -> Option<MessageMetadata<C>>
  where
    C: MetadataStorageMode, {
    self.with_inner(|inner| inner.take(key))
  }

  pub fn discard(&self, key: MetadataKey) {
    self.with_inner(|inner| {
      inner.discard(key);
    });
  }
}

#[cfg(not(target_has_atomic = "ptr"))]
fn global_table() -> &'static MetadataTable {
  static TABLE: MetadataTable = MetadataTable::new();
  &TABLE
}

/// Stores a value in the global metadata table and returns its key.
pub fn store_metadata<C>(metadata: MessageMetadata<C>) -> MetadataKey
where
  C: MetadataStorageMode, {
  global_table().store(metadata)
}

/// Retrieves previously registered metadata and removes it from the table.
pub fn take_metadata<C>(key: MetadataKey) -> Option<MessageMetadata<C>>
where
  C: MetadataStorageMode, {
  global_table().take(key)
}

/// Drops metadata associated with the specified key without attempting to downcast it.
pub fn discard_metadata(key: MetadataKey) {
  global_table().discard(key);
}
