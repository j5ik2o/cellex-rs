#![allow(clippy::disallowed_types)]
use alloc::{boxed::Box, rc::Rc};

use async_trait::async_trait;
use cellex_utils_core_rs::SynchronizedRwBackend;
use embassy_sync::{
  blocking_mutex::raw::NoopRawMutex,
  rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// `Rc` + `RwLock` read/write synchronization backend.
///
/// Implements a synchronization primitive that provides multiple readers or single writer access in
/// `no_std` environments. Uses Embassy's `RwLock` to achieve asynchronous read/write access to
/// values.
///
/// # Features
///
/// - Reference counting via `Rc` (single-threaded only)
/// - Lightweight lock mechanism via Embassy's `NoopRawMutex`
/// - Allows multiple readers or a single writer
#[derive(Clone, Debug)]
pub struct RcRwLockBackend<T> {
  inner: Rc<RwLock<NoopRawMutex, T>>,
}

#[async_trait(?Send)]
impl<T> SynchronizedRwBackend<T> for RcRwLockBackend<T>
where
  T: 'static,
{
  type ReadGuard<'a>
    = RwLockReadGuard<'a, NoopRawMutex, T>
  where
    Self: 'a;
  type WriteGuard<'a>
    = RwLockWriteGuard<'a, NoopRawMutex, T>
  where
    Self: 'a;

  /// Creates a new read/write synchronization backend with the specified value.
  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: Rc::new(RwLock::new(value)) }
  }

  /// Acquires a read lock and returns a read guard.
  ///
  /// Multiple read locks can be held simultaneously.
  /// Waits until a write lock is released if one is held.
  async fn read(&self) -> Self::ReadGuard<'_> {
    self.inner.read().await
  }

  /// Acquires a write lock and returns a write guard.
  ///
  /// Write locks are exclusive and wait until all other read/write locks are released.
  async fn write(&self) -> Self::WriteGuard<'_> {
    self.inner.write().await
  }
}
