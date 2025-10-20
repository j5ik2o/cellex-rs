use core::time::Duration;

use cellex_utils_core_rs::SendBound;

/// Scheduler abstraction for managing actor `ReceiveTimeout`.
///
/// Provides a unified interface for setting/resetting/stopping timeouts,
/// so that `actor-core` doesn't need to directly handle runtime-dependent timers.
/// By calling `notify_activity` after user message processing,
/// the runtime side can re-arm with any implementation (tokio / embedded software timer, etc.).
pub trait ReceiveTimeoutScheduler: SendBound {
  /// Sets/re-arms the timer with the specified duration.
  fn set(&mut self, duration: Duration);

  /// Stops the timer.
  fn cancel(&mut self);

  /// Notifies of activity (like user messages) that should reset the timeout.
  fn notify_activity(&mut self);
}
