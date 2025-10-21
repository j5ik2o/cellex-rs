//! Wrapper around `std::sync::Mutex` implementing the core `MutexLike` trait.

use cellex_utils_core_rs::sync::sync_mutex_like::SyncMutexLike;

/// Thin wrapper over [`std::sync::Mutex`] for synchronous std environments.
pub struct StdSyncMutex<T>(std::sync::Mutex<T>);

impl<T> StdSyncMutex<T> {
  /// Creates a new mutex guarding the provided value.
  #[must_use]
  pub fn new(value: T) -> Self {
    Self(std::sync::Mutex::new(value))
  }

  /// Consumes the mutex and returns the inner value.
  pub fn into_inner(self) -> T {
    self.0.into_inner().unwrap_or_else(|err| err.into_inner())
  }

  /// Returns a reference to the underlying `std::sync::Mutex`.
  #[must_use]
  pub fn as_inner(&self) -> &std::sync::Mutex<T> {
    &self.0
  }

  /// Locks the mutex and returns the guard.
  pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
    self.0.lock().unwrap_or_else(|err| err.into_inner())
  }
}

impl<T> SyncMutexLike<T> for StdSyncMutex<T> {
  type Guard<'a>
    = std::sync::MutexGuard<'a, T>
  where
    T: 'a;

  fn new(value: T) -> Self {
    StdSyncMutex::new(value)
  }

  fn into_inner(self) -> T {
    StdSyncMutex::into_inner(self)
  }

  fn lock(&self) -> Self::Guard<'_> {
    StdSyncMutex::lock(self)
  }
}

/// Convenience alias for guards produced by [`StdSyncMutex`].
pub type StdMutexGuard<'a, T> = std::sync::MutexGuard<'a, T>;
