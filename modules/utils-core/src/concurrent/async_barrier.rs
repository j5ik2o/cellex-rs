use alloc::boxed::Box;

use async_trait::async_trait;

/// Trait defining the backend implementation for async barriers.
///
/// This trait is used to implement barrier synchronization backends
/// for different execution environments (Tokio, async-std, etc.).
///
/// # Implementation Requirements
///
/// - Must implement the `Clone` trait
/// - Need not be thread-sendable (`?Send`)
#[async_trait(?Send)]
pub trait AsyncBarrierBackend: Clone {
  /// Creates a backend that waits for the specified number of tasks.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of tasks that must reach the barrier before it is released
  fn new(count: usize) -> Self;

  /// Waits at the barrier point.
  ///
  /// Tasks calling this method block until all tasks (specified count)
  /// have called `wait()`.
  /// When all tasks arrive, all tasks are released simultaneously.
  async fn wait(&self);
}

/// Structure providing synchronization barrier among async tasks.
///
/// `AsyncBarrier` provides a barrier synchronization mechanism for multiple async
/// tasks to synchronize at a specific point. All tasks wait until the specified
/// number of tasks reach the barrier, then all resume processing simultaneously.
///
/// # Type Parameters
///
/// * `B` - Backend implementation to use (a type implementing `AsyncBarrierBackend`)
#[derive(Clone, Debug)]
pub struct AsyncBarrier<B>
where
  B: AsyncBarrierBackend, {
  backend: B,
}

impl<B> AsyncBarrier<B>
where
  B: AsyncBarrierBackend,
{
  /// Creates a new barrier that waits for the specified number of tasks.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of tasks that must reach the barrier before it is released
  ///
  /// # Returns
  ///
  /// A new `AsyncBarrier` instance
  ///
  /// # Panics
  ///
  /// If `count` is 0, some backend implementations may panic.
  #[must_use]
  pub fn new(count: usize) -> Self {
    Self { backend: B::new(count) }
  }

  /// Waits at the barrier point.
  ///
  /// Tasks calling this method enter a wait state until all tasks (the specified count)
  /// have called `wait()` on the barrier.
  /// When all tasks arrive, all tasks are released simultaneously and can
  /// continue processing.
  ///
  /// # Behavior
  ///
  /// - The first `count - 1` tasks enter a wait state when calling `wait()`
  /// - When the `count`-th task calls `wait()`, all tasks are released simultaneously
  /// - The barrier may be reusable depending on the backend implementation
  pub async fn wait(&self) {
    self.backend.wait().await;
  }

  /// Gets a reference to the backend implementation.
  ///
  /// This method is used when you need to access backend-specific functionality.
  /// For normal usage, the `new()` and `wait()` methods are sufficient.
  ///
  /// # Returns
  ///
  /// Immutable reference to the backend implementation
  pub const fn backend(&self) -> &B {
    &self.backend
  }
}
