use cellex_utils_core_rs::{Synchronized as CoreSynchronized, SynchronizedRw as CoreSynchronizedRw};

mod rc_mutex_backend;
mod rc_rw_lock_backend;

pub use rc_mutex_backend::RcMutexBackend;
pub use rc_rw_lock_backend::RcRwLockBackend;

/// Type alias for `Rc`-based exclusive synchronization type.
///
/// Synchronization primitive usable in `no_std` environments that provides exclusive access
/// control. Uses the same lock for both reads and writes.
pub type Synchronized<T> = CoreSynchronized<RcMutexBackend<T>, T>;

/// Type alias for `Rc`-based read/write synchronization type.
///
/// Synchronization primitive usable in `no_std` environments that provides multiple readers or
/// single writer access.
pub type SynchronizedRw<T> = CoreSynchronizedRw<RcRwLockBackend<T>, T>;

#[cfg(test)]
mod tests;
