#![allow(clippy::disallowed_types)]

use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use cellex_utils_core_rs::{SynchronizedRw as CoreSynchronizedRw, SynchronizedRwBackend};
use embassy_sync::{
  blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
  rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// Backend implementation for read-write lock synchronization using `Arc`
///
/// Provides concurrent read access with exclusive write access using embassy-sync's
/// `RwLock` with `Arc` for thread-safe reference counting.
///
/// # Type Parameters
///
/// * `RM` - Raw mutex type from embassy-sync
/// * `T` - The value type being synchronized
#[derive(Clone, Debug)]
pub struct ArcRwLockBackend<RM, T>
where
  RM: RawMutex, {
  inner: Arc<RwLock<RM, T>>,
}

#[async_trait(?Send)]
impl<RM, T> SynchronizedRwBackend<T> for ArcRwLockBackend<RM, T>
where
  RM: RawMutex,
  T: Send,
{
  type ReadGuard<'a>
    = RwLockReadGuard<'a, RM, T>
  where
    Self: 'a;
  type WriteGuard<'a>
    = RwLockWriteGuard<'a, RM, T>
  where
    Self: 'a;

  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: Arc::new(RwLock::new(value)) }
  }

  async fn read(&self) -> Self::ReadGuard<'_> {
    self.inner.read().await
  }

  async fn write(&self) -> Self::WriteGuard<'_> {
    self.inner.write().await
  }
}

/// Type alias for `Arc`-based read-write lock synchronization
///
/// Provides concurrent reads with exclusive writes using configurable mutex backend.
pub type ArcSynchronizedRw<T, RM> = CoreSynchronizedRw<ArcRwLockBackend<RM, T>, T>;

/// Type alias for `ArcSynchronizedRw` using `CriticalSectionRawMutex`
///
/// Provides interrupt-safe read-write lock for embedded contexts.
pub type ArcLocalSynchronizedRw<T> = ArcSynchronizedRw<T, CriticalSectionRawMutex>;

/// Alias for `ArcLocalSynchronizedRw` for consistency
///
/// Uses critical section rwlock backend.
pub type ArcCsSynchronizedRw<T> = ArcLocalSynchronizedRw<T>;
