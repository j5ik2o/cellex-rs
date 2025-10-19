use core::sync::atomic::{AtomicBool, Ordering};

use cellex_utils_core_rs::ArcShared;

/// Token that controls shutdown of the actor system.
///
/// Can be shared among multiple threads or tasks and cooperatively manages shutdown state.
#[derive(Clone)]
pub struct ShutdownToken {
  inner: ArcShared<AtomicBool>,
}

impl ShutdownToken {
  /// Creates a new shutdown token.
  ///
  /// Shutdown is not triggered in the initial state.
  ///
  /// # Returns
  /// New shutdown token
  #[must_use]
  pub fn new() -> Self {
    Self { inner: ArcShared::new(AtomicBool::new(false)) }
  }

  /// Triggers shutdown.
  ///
  /// This operation can be safely called from multiple threads.
  /// Once triggered, the state cannot be reset.
  pub fn trigger(&self) {
    self.inner.store(true, Ordering::SeqCst);
  }

  /// Checks whether shutdown has been triggered.
  ///
  /// # Returns
  /// `true` if shutdown has been triggered, `false` otherwise
  #[must_use]
  pub fn is_triggered(&self) -> bool {
    self.inner.load(Ordering::SeqCst)
  }
}

impl Default for ShutdownToken {
  fn default() -> Self {
    Self::new()
  }
}
