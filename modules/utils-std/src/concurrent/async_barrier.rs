mod tokio_async_barrier_backend;

#[cfg(test)]
mod tests;

use cellex_utils_core_rs::AsyncBarrier as CoreAsyncBarrier;
pub use tokio_async_barrier_backend::TokioAsyncBarrierBackend;

/// Async barrier using Tokio runtime
///
/// A synchronization primitive that causes all tasks to wait until the specified number of tasks
/// arrive. When all tasks reach the barrier, it resets to a reusable state.
pub type AsyncBarrier = CoreAsyncBarrier<TokioAsyncBarrierBackend>;
