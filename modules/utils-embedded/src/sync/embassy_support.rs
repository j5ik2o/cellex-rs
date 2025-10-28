use alloc::boxed::Box;
use core::marker::PhantomData;

use async_trait::async_trait;
use cellex_utils_core_rs::sync::{
  async_mutex_like::AsyncMutexLike,
  interrupt::{InterruptContextPolicy, NeverInterruptPolicy},
  SharedError,
};
use embassy_sync::{
  blocking_mutex::raw::RawMutex,
  mutex::{Mutex, MutexGuard},
};

/// Async-aware mutex wrapper backed by `embassy_sync::mutex::Mutex`.
pub struct EmbassyAsyncMutex<M, T, P = NeverInterruptPolicy>
where
  M: RawMutex,
  P: InterruptContextPolicy, {
  inner:   Mutex<M, T>,
  _policy: PhantomData<P>,
}

impl<M, T, P> EmbassyAsyncMutex<M, T, P>
where
  M: RawMutex,
  P: InterruptContextPolicy,
{
  /// Creates a new mutex guarding the provided value.
  pub const fn new(value: T) -> Self {
    Self { inner: Mutex::new(value), _policy: PhantomData }
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
impl<M, T, P> AsyncMutexLike<T> for EmbassyAsyncMutex<M, T, P>
where
  M: RawMutex,
  P: InterruptContextPolicy + Send + Sync,
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

  async fn lock(&self) -> Result<Self::Guard<'_>, SharedError> {
    P::check_blocking_allowed()?;
    Ok(self.inner.lock().await)
  }
}

/// Alias for the guard type returned by [`EmbassyAsyncMutex`].
pub type EmbassyAsyncMutexGuard<'a, M, T> = MutexGuard<'a, M, T>;
