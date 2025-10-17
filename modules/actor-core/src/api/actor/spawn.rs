use core::future::Future;

/// Interface for abstracting asynchronous task execution.
///
/// Provides an abstraction layer for spawning asynchronous tasks in an environment-independent manner.
pub trait Spawn {
  /// Spawns a new asynchronous task.
  ///
  /// # Arguments
  ///
  /// * `fut` - Asynchronous task to execute
  fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static);
}
