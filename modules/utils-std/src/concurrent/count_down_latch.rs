mod tokio_count_down_latch_backend;

#[cfg(test)]
mod tests;

use cellex_utils_core_rs::CountDownLatch as CoreCountDownLatch;
pub use tokio_count_down_latch_backend::TokioCountDownLatchBackend;

/// Countdown latch using Tokio runtime
///
/// A synchronization primitive that causes tasks to wait until the specified number of countdowns
/// complete. When `count_down()` is called as many times as the initial count, all tasks waiting on
/// `wait()` are released.
pub type CountDownLatch = CoreCountDownLatch<TokioCountDownLatchBackend>;
