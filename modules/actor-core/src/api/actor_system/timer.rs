use core::future::Future;
use core::time::Duration;

/// Generic timer abstraction.
///
/// Provides an abstraction layer for delayed execution in an environment-independent manner.
pub trait Timer {
  /// Future type for sleep operation
  type SleepFuture<'a>: Future<Output = ()> + 'a
  where
    Self: 'a;

  /// Returns a Future that sleeps for the specified duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - Duration to sleep
  fn sleep(&self, duration: Duration) -> Self::SleepFuture<'_>;
}
