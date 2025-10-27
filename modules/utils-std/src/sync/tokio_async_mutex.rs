//! Tokio-specific synchronization adaptors for v2 abstractions.

use async_trait::async_trait;
use cellex_utils_core_rs::sync::{
  async_mutex_like::AsyncMutexLike,
  interrupt::{InterruptContextPolicy, NeverInterruptPolicy},
  SharedError,
};

/// Wrapper around [`tokio::sync::Mutex`] that implements [`AsyncMutexLike`].
pub struct TokioAsyncMutex<T>(tokio::sync::Mutex<T>);

impl<T> TokioAsyncMutex<T> {
  /// Creates a new mutex protecting the given value.
  pub fn new(value: T) -> Self {
    Self(tokio::sync::Mutex::new(value))
  }

  /// Consumes the mutex and returns the inner value.
  pub fn into_inner(self) -> T {
    self.0.into_inner()
  }

  /// Returns a shared reference to the underlying Tokio mutex.
  #[must_use]
  pub fn as_inner(&self) -> &tokio::sync::Mutex<T> {
    &self.0
  }

  /// Acquires the mutex asynchronously.
  pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, T> {
    self.0.lock().await
  }
}

#[async_trait(?Send)]
impl<T: Send> AsyncMutexLike<T> for TokioAsyncMutex<T> {
  type Guard<'a>
    = tokio::sync::MutexGuard<'a, T>
  where
    T: 'a;

  fn new(value: T) -> Self {
    Self::new(value)
  }

  fn into_inner(self) -> T {
    self.into_inner()
  }

  async fn lock(&self) -> Result<Self::Guard<'_>, SharedError> {
    NeverInterruptPolicy::check_blocking_allowed()?;
    Ok(self.0.lock().await)
  }
}

/// Convenience alias for guards produced by [`TokioAsyncMutex`].
pub type TokioMutexGuard<'a, T> = tokio::sync::MutexGuard<'a, T>;
