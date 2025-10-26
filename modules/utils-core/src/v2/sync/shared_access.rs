//! Access helpers for shared backends guarded by mutex-like primitives.

use super::SharedError;
use crate::sync::{shared::Shared, sync_mutex_like::SyncMutexLike, ArcShared};

/// Abstraction offering mutable access to shared backends.
pub trait SharedAccess<B> {
  /// Executes the provided closure with mutable access to the backend.
  fn with_mut<R>(&self, f: impl FnOnce(&mut B) -> R) -> Result<R, SharedError>;
}

impl<B, M> SharedAccess<B> for ArcShared<M>
where
  M: SyncMutexLike<B>,
{
  fn with_mut<R>(&self, f: impl FnOnce(&mut B) -> R) -> Result<R, SharedError> {
    let result = self.with_ref(|mutex| {
      let mut guard = mutex.lock();
      f(&mut guard)
    });
    Ok(result)
  }
}
