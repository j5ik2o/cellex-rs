//! Generic synchronized primitives.

pub mod guard_handle;
pub mod synchronized_mutex;
pub mod synchronized_mutex_backend;
pub mod synchronized_rw;
pub mod synchronized_rw_backend;

pub use guard_handle::GuardHandle;
pub use synchronized_mutex::Synchronized;
pub use synchronized_mutex_backend::SynchronizedMutexBackend;
pub use synchronized_rw::SynchronizedRw;
pub use synchronized_rw_backend::SynchronizedRwBackend;
