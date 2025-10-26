//! Tokio Mutex backend implementation.

use async_trait::async_trait;
use cellex_utils_core_rs::{
  concurrent::synchronized::SynchronizedMutexBackend,
  sync::interrupt::{InterruptContextPolicy, NeverInterruptPolicy},
  v2::sync::SharedError,
};
use tokio::sync::{Mutex, MutexGuard};

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

  async fn lock(&self) -> Result<Self::Guard<'_>, SharedError> {
    NeverInterruptPolicy::check_blocking_allowed()?;
    Ok(self.inner.lock().await)
  }
}
