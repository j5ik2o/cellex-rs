#[cfg(test)]
mod tests;

use async_trait::async_trait;
use cellex_utils_core_rs::{
  Synchronized as CoreSynchronized, SynchronizedMutexBackend, SynchronizedRw as CoreSynchronizedRw,
  SynchronizedRwBackend,
};
use tokio::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Backend implementation of exclusive control using Tokio Mutex
///
/// Provides exclusive access to shared data.
pub struct TokioMutexBackend<T> {
  inner: Mutex<T>,
}

impl<T> TokioMutexBackend<T> {
  /// Creates a new backend instance from an existing Tokio Mutex.
  ///
  /// # Arguments
  ///
  /// * `inner` - The Tokio Mutex to wrap
  ///
  /// # Returns
  ///
  /// A new `TokioMutexBackend` instance
  pub const fn new_with_mutex(inner: Mutex<T>) -> Self {
    Self { inner }
  }
}

#[async_trait(?Send)]
impl<T> SynchronizedMutexBackend<T> for TokioMutexBackend<T>
where
  T: Send,
{
  type Guard<'a>
    = MutexGuard<'a, T>
  where
    Self: 'a;

  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: Mutex::new(value) }
  }

  async fn lock(&self) -> Self::Guard<'_> {
    self.inner.lock().await
  }
}

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

/// Shared data with exclusive control using Tokio runtime
///
/// Provides exclusive access via `Mutex`, allowing safe data sharing across multiple tasks.
pub type Synchronized<T> = CoreSynchronized<TokioMutexBackend<T>, T>;

/// Shared data with read-write lock using Tokio runtime
///
/// Provides read/write access via `RwLock`, allowing multiple reads or a single write.
pub type SynchronizedRw<T> = CoreSynchronizedRw<TokioRwLockBackend<T>, T>;
