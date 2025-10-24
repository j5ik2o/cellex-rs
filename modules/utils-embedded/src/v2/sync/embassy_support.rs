use alloc::boxed::Box;

use async_trait::async_trait;
use cellex_utils_core_rs::sync::async_mutex_like::AsyncMutexLike;
use embassy_sync::{
  blocking_mutex::raw::RawMutex,
  mutex::{Mutex, MutexGuard},
};

/// Async-aware mutex wrapper backed by `embassy_sync::mutex::Mutex`.
pub struct EmbassyAsyncMutex<M, T>
where
  M: RawMutex, {
  inner: Mutex<M, T>,
}

impl<M, T> EmbassyAsyncMutex<M, T>
where
  M: RawMutex,
{
  /// Creates a new mutex guarding the provided value.
  pub const fn new(value: T) -> Self {
    Self { inner: Mutex::new(value) }
  }

  /// Consumes the mutex wrapper and returns the protected value.
  pub fn into_inner(self) -> T
  where
    T: Sized, {
    self.inner.into_inner()
  }

  /// Borrows the underlying Embassy mutex.
  #[must_use]
  pub fn as_inner(&self) -> &Mutex<M, T> {
    &self.inner
  }
}

#[async_trait(?Send)]
impl<M, T> AsyncMutexLike<T> for EmbassyAsyncMutex<M, T>
where
  M: RawMutex,
{
  type Guard<'a>
    = MutexGuard<'a, M, T>
  where
    Self: 'a,
    T: 'a;

  fn new(value: T) -> Self {
    Self::new(value)
  }

  fn into_inner(self) -> T
  where
    T: Sized, {
    EmbassyAsyncMutex::into_inner(self)
  }

  async fn lock(&self) -> Self::Guard<'_> {
    self.inner.lock().await
  }
}

/// Alias for the guard type returned by [`EmbassyAsyncMutex`].
pub type EmbassyAsyncMutexGuard<'a, M, T> = MutexGuard<'a, M, T>;
