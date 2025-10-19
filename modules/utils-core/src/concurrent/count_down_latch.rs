use alloc::boxed::Box;

use async_trait::async_trait;

/// Trait defining the backend implementation for CountDownLatch
///
/// This trait abstracts the concrete implementation of CountDownLatch to support
/// different environments (standard library, embedded environments, etc.).
#[async_trait(?Send)]
pub trait CountDownLatchBackend: Clone {
  /// Initializes the backend with the specified count value
  ///
  /// # Arguments
  ///
  /// * `count` - Initial count value. Counts down until it reaches 0
  fn new(count: usize) -> Self;

  /// Decrements the count by 1
  ///
  /// When the count reaches 0, all waiting tasks are released.
  async fn count_down(&self);

  /// Waits until the count reaches 0
  ///
  /// This method blocks the current task until the count becomes 0.
  /// Multiple tasks can wait simultaneously.
  async fn wait(&self);
}

/// Count-down latch synchronization primitive
///
/// `CountDownLatch` is a synchronization mechanism that causes multiple tasks to wait
/// until a specified count reaches zero. It provides functionality equivalent to Java's
/// `CountDownLatch` or Go's `WaitGroup`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountDownLatch<B>
where
  B: CountDownLatchBackend, {
  backend: B,
}

impl<B> CountDownLatch<B>
where
  B: CountDownLatchBackend,
{
  /// Creates a new `CountDownLatch` with the specified count value
  ///
  /// # Arguments
  ///
  /// * `count` - Initial count value. `count_down` must be called this many times to reach 0
  #[must_use]
  pub fn new(count: usize) -> Self {
    Self { backend: B::new(count) }
  }

  /// Decrements the count by 1
  ///
  /// When the count reaches 0, all tasks waiting in `wait()` are released.
  /// If the count is already 0, this method does nothing.
  pub async fn count_down(&self) {
    self.backend.count_down().await;
  }

  /// Causes the current task to wait until the count reaches 0
  ///
  /// If the count is already 0, this method returns immediately.
  /// Multiple tasks can wait simultaneously, and all are released at once when the count reaches 0.
  pub async fn wait(&self) {
    self.backend.wait().await;
  }

  /// Gets a reference to the internal backend
  ///
  /// # Returns
  ///
  /// Immutable reference to the backend implementation
  pub const fn backend(&self) -> &B {
    &self.backend
  }
}

impl<B> Default for CountDownLatch<B>
where
  B: CountDownLatchBackend,
{
  /// Creates a default `CountDownLatch` with a count of 0
  ///
  /// This default implementation creates a latch with count 0.
  /// This means `wait()` will return immediately.
  fn default() -> Self {
    Self::new(0)
  }
}
