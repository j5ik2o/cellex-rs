use alloc::boxed::Box;

use async_trait::async_trait;

use crate::sync::async_mutex_like::AsyncMutexLike;

/// Thin wrapper around [`spin::Mutex`] implementing [`AsyncMutexLike`].
#[allow(dead_code)]
pub struct SpinAsyncMutex<T>(spin::Mutex<T>);

#[allow(dead_code)]
impl<T> SpinAsyncMutex<T> {
  /// Creates a new spinlock-protected value.
  #[must_use]
  pub const fn new(value: T) -> Self {
    Self(spin::Mutex::new(value))
  }

  /// Returns a reference to the inner spin mutex.
  #[must_use]
  pub const fn as_inner(&self) -> &spin::Mutex<T> {
    &self.0
  }

  /// Consumes the wrapper and returns the underlying value.
  pub fn into_inner(self) -> T {
    self.0.into_inner()
  }

  /// Locks the mutex and returns a guard to the protected value.
  pub fn lock(&self) -> spin::MutexGuard<'_, T> {
    self.0.lock()
  }
}

#[async_trait(?Send)]
impl<T> AsyncMutexLike<T> for SpinAsyncMutex<T> {
  type Guard<'a>
    = spin::MutexGuard<'a, T>
  where
    T: 'a;

  fn new(value: T) -> Self {
    SpinAsyncMutex::new(value)
  }

  fn into_inner(self) -> T {
    SpinAsyncMutex::into_inner(self)
  }

  async fn lock(&self) -> Self::Guard<'_> {
    SpinAsyncMutex::lock(self)
  }
}
