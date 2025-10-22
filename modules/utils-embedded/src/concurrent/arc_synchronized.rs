#![allow(clippy::disallowed_types)]
#![cfg(feature = "arc")]

mod arc_rw_lock_backend;
#[cfg(all(test, feature = "std"))]
mod tests;

use alloc::{boxed::Box, sync::Arc};

pub use arc_rw_lock_backend::{ArcCsSynchronizedRw, ArcLocalSynchronizedRw, ArcRwLockBackend, ArcSynchronizedRw};
use async_trait::async_trait;
use cellex_utils_core_rs::{Synchronized as CoreSynchronized, SynchronizedMutexBackend};
use embassy_sync::{
  blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
  mutex::{Mutex, MutexGuard},
};

/// Backend implementation for mutex-based synchronization using `Arc`
///
/// Provides exclusive-access synchronization using embassy-sync's `Mutex` with
/// `Arc` for thread-safe reference counting.
///
/// # Type Parameters
///
/// * `RM` - Raw mutex type from embassy-sync
/// * `T` - The value type being synchronized
#[derive(Clone, Debug)]
pub struct ArcMutexBackend<RM, T>
where
  RM: RawMutex, {
  inner: Arc<Mutex<RM, T>>,
}

#[async_trait(?Send)]
impl<RM, T> SynchronizedMutexBackend<T> for ArcMutexBackend<RM, T>
where
  RM: RawMutex,
  T: Send,
{
  type Guard<'a>
    = MutexGuard<'a, RM, T>
  where
    Self: 'a;

  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: Arc::new(Mutex::new(value)) }
  }

  async fn lock(&self) -> Self::Guard<'_> {
    self.inner.lock().await
  }
}

/// Type alias for `Arc`-based mutex synchronization
///
/// Provides exclusive-access synchronization with configurable mutex backend.
pub type ArcSynchronized<T, RM> = CoreSynchronized<ArcMutexBackend<RM, T>, T>;

/// Type alias for `ArcSynchronized` using `CriticalSectionRawMutex`
///
/// Provides interrupt-safe critical section protection for embedded contexts.
pub type ArcLocalSynchronized<T> = ArcSynchronized<T, CriticalSectionRawMutex>;

/// Alias for `ArcLocalSynchronized` for consistency
///
/// Uses critical section mutex backend.
pub type ArcCsSynchronized<T> = ArcLocalSynchronized<T>;
