#![allow(clippy::disallowed_types)]
#![cfg(feature = "arc")]

mod arc_rw_lock_backend;
#[cfg(all(test, feature = "arc"))]
mod tests;

use alloc::{boxed::Box, sync::Arc};
use core::marker::PhantomData;

pub use arc_rw_lock_backend::{ArcCsSynchronizedRw, ArcLocalSynchronizedRw, ArcRwLockBackend, ArcSynchronizedRw};
use async_trait::async_trait;
use cellex_utils_core_rs::{
  concurrent::synchronized::{Synchronized as CoreSynchronized, SynchronizedMutexBackend},
  sync::interrupt::{CriticalSectionInterruptPolicy, InterruptContextPolicy, NeverInterruptPolicy},
  v2::sync::SharedError,
};
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
pub struct ArcMutexBackend<RM, T, P = NeverInterruptPolicy>
where
  RM: RawMutex,
  P: InterruptContextPolicy, {
  inner:   Arc<Mutex<RM, T>>,
  _policy: PhantomData<P>,
}

#[async_trait(?Send)]
impl<RM, T, P> SynchronizedMutexBackend<T> for ArcMutexBackend<RM, T, P>
where
  RM: RawMutex,
  T: Send,
  P: InterruptContextPolicy + Send + Sync,
{
  type Guard<'a>
    = MutexGuard<'a, RM, T>
  where
    Self: 'a;

  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: Arc::new(Mutex::new(value)), _policy: PhantomData }
  }

  async fn lock(&self) -> Result<Self::Guard<'_>, SharedError> {
    P::check_blocking_allowed()?;
    Ok(self.inner.lock().await)
  }
}

/// Type alias for `Arc`-based mutex synchronization
///
/// Provides exclusive-access synchronization with configurable mutex backend.
pub type ArcSynchronized<T, RM> = CoreSynchronized<ArcMutexBackend<RM, T, NeverInterruptPolicy>, T>;

/// Type alias for `ArcSynchronized` using `CriticalSectionRawMutex`
///
/// Provides interrupt-safe critical section protection for embedded contexts.
pub type ArcLocalSynchronized<T> =
  CoreSynchronized<ArcMutexBackend<CriticalSectionRawMutex, T, CriticalSectionInterruptPolicy>, T>;

/// Alias for `ArcLocalSynchronized` for consistency
///
/// Uses critical section mutex backend.
pub type ArcCsSynchronized<T> = ArcLocalSynchronized<T>;
