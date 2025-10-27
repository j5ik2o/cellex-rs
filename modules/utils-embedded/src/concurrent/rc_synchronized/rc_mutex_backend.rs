#![allow(clippy::disallowed_types)]
use alloc::{boxed::Box, rc::Rc};

use async_trait::async_trait;
use cellex_utils_core_rs::{
  concurrent::synchronized::SynchronizedMutexBackend,
  sync::{
    interrupt::{InterruptContextPolicy, NeverInterruptPolicy},
    SharedError,
  },
};
use embassy_sync::{
  blocking_mutex::raw::NoopRawMutex,
  mutex::{Mutex, MutexGuard},
};

/// `Rc` + `Mutex` synchronization backend.
///
/// Implements a synchronization primitive that provides exclusive access in `no_std` environments.
/// Uses Embassy's `Mutex` to achieve asynchronous exclusive access to values.
///
/// # Features
///
/// - Reference counting via `Rc` (single-threaded only)
/// - Lightweight exclusive control via Embassy's `NoopRawMutex`
/// - Uses the same lock for both reads and writes
#[derive(Clone, Debug)]
pub struct RcMutexBackend<T> {
  inner: Rc<Mutex<NoopRawMutex, T>>,
}

#[async_trait(?Send)]
impl<T> SynchronizedMutexBackend<T> for RcMutexBackend<T>
where
  T: 'static,
{
  type Guard<'a>
    = MutexGuard<'a, NoopRawMutex, T>
  where
    Self: 'a;

  /// Creates a new synchronization backend with the specified value.
  fn new(value: T) -> Self
  where
    T: Sized, {
    Self { inner: Rc::new(Mutex::new(value)) }
  }

  /// Acquires the lock and returns a guard.
  ///
  /// Waits until the lock is released if another task holds it.
  async fn lock(&self) -> Result<Self::Guard<'_>, SharedError> {
    NeverInterruptPolicy::check_blocking_allowed()?;
    Ok(self.inner.lock().await)
  }
}
