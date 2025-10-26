//! Tokio RwLock backend implementation.

use async_trait::async_trait;
use cellex_utils_core_rs::concurrent::synchronized::SynchronizedRwBackend;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Backend implementation of read-write lock using Tokio RwLock
///
/// Provides multiple read accesses or a single write access.
pub struct TokioRwLockBackend<T> {
  inner: RwLock<T>,
}

impl<T> TokioRwLockBackend<T> {
  /// Creates a new backend instance from an existing Tokio RwLock.
  ///
  /// # Arguments
  ///
  /// * `inner` - The Tokio RwLock to wrap
  ///
  /// # Returns
  ///
  /// A new `TokioRwLockBackend` instance
  pub const fn new_with_rwlock(inner: RwLock<T>) -> Self {
    Self { inner }
  }
}

#[async_trait(?Send)]
impl<T> SynchronizedRwBackend<T> for TokioRwLockBackend<T>
where
  T: Send + Sync,
{
  type ReadGuard<'a>
    = RwLockReadGuard<'a, T>
  where
    Self: 'a;
  type WriteGuard<'a>
    = RwLockWriteGuard<'a, T>
  where
    Self: 'a;

  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: RwLock::new(value) }
  }

  async fn read(&self) -> Self::ReadGuard<'_> {
    self.inner.read().await
  }

  async fn write(&self) -> Self::WriteGuard<'_> {
    self.inner.write().await
  }
}
