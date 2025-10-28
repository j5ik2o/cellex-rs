mod tokio_mutex_backend;
mod tokio_rw_lock_backend;

#[cfg(test)]
mod tests;

use cellex_utils_core_rs::concurrent::synchronized::{
  Synchronized as CoreSynchronized, SynchronizedRw as CoreSynchronizedRw,
};
pub use tokio_mutex_backend::TokioMutexBackend;
pub use tokio_rw_lock_backend::TokioRwLockBackend;

/// Shared data with exclusive control using Tokio runtime
///
/// Provides exclusive access via `Mutex`, allowing safe data sharing across multiple tasks.
pub type Synchronized<T> = CoreSynchronized<TokioMutexBackend<T>, T>;

/// Shared data with read-write lock using Tokio runtime
///
/// Provides read/write access via `RwLock`, allowing multiple reads or a single write.
pub type SynchronizedRw<T> = CoreSynchronizedRw<TokioRwLockBackend<T>, T>;
