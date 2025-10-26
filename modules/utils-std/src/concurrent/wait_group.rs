mod tokio_wait_group_backend;

#[cfg(test)]
mod tests;

use cellex_utils_core_rs::concurrent::wait_group::WaitGroup as CoreWaitGroup;
pub use tokio_wait_group_backend::TokioWaitGroupBackend;

/// Type alias for WaitGroup using Tokio backend
///
/// A synchronization primitive for waiting until multiple async tasks complete.
pub type WaitGroup = CoreWaitGroup<TokioWaitGroupBackend>;
