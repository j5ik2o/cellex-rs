//! AdaptiveCoordinator - Type alias for DefaultReadyQueueCoordinatorV2
//!
//! This module provides a coordinator type alias that was previously used for
//! selecting between different implementations. Now it simply uses the default
//! Mutex-based implementation.
//!
//! # Note
//!
//! Lock-free implementations have been moved to cellex-actor-std-rs to maintain
//! actor-core's no_std compatibility.

use alloc::vec::Vec;
use core::task::{Context, Poll};

use super::{DefaultReadyQueueCoordinatorV2, InvokeResult, MailboxIndex, ReadyQueueCoordinatorV2};

#[cfg(test)]
mod tests;

/// Adaptive coordinator - currently a simple wrapper around DefaultReadyQueueCoordinatorV2
///
/// # Examples
///
/// ```rust,no_run
/// # extern crate alloc;
/// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
/// let coordinator = AdaptiveCoordinator::new(32, 4);
/// ```
///
/// # Note
///
/// This was previously an enum that could select between different implementations.
/// Lock-free variants have been moved to cellex-actor-std-rs for std environments.
pub struct AdaptiveCoordinator(DefaultReadyQueueCoordinatorV2);

impl AdaptiveCoordinator {
  /// Create a new adaptive coordinator
  ///
  /// # Arguments
  ///
  /// * `throughput` - Messages per invocation hint
  /// * `concurrency_hint` - Expected number of concurrent worker threads (currently unused)
  ///
  /// # Note
  ///
  /// The concurrency_hint parameter is kept for API compatibility but is currently
  /// unused as only the Mutex-based implementation is available in no_std.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # extern crate alloc;
  /// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
  /// let coordinator = AdaptiveCoordinator::new(32, 4);
  /// ```
  pub fn new(throughput: usize, _concurrency_hint: usize) -> Self {
    Self(DefaultReadyQueueCoordinatorV2::new(throughput))
  }

  /// Create a coordinator with explicit implementation selection
  ///
  /// # Arguments
  ///
  /// * `throughput` - Messages per invocation hint
  /// * `use_lockfree` - If true, panics (lock-free not available in no_std)
  ///
  /// # Panics
  ///
  /// Panics if `use_lockfree` is true (lock-free implementations are only available
  /// in cellex-actor-std-rs)
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # extern crate alloc;
  /// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
  /// // Use default implementation
  /// let coordinator = AdaptiveCoordinator::with_strategy(32, false);
  /// ```
  pub fn with_strategy(throughput: usize, use_lockfree: bool) -> Self {
    if use_lockfree {
      panic!("lock-free coordinators are only available in cellex-actor-std-rs");
    }
    Self(DefaultReadyQueueCoordinatorV2::new(throughput))
  }
}

impl ReadyQueueCoordinatorV2 for AdaptiveCoordinator {
  fn register_ready(&self, idx: MailboxIndex) {
    self.0.register_ready(idx)
  }

  fn unregister(&self, idx: MailboxIndex) {
    self.0.unregister(idx)
  }

  fn drain_ready_cycle(&self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
    self.0.drain_ready_cycle(max_batch, out)
  }

  fn poll_wait_signal(&self, cx: &mut Context<'_>) -> Poll<()> {
    self.0.poll_wait_signal(cx)
  }

  fn handle_invoke_result(&self, idx: MailboxIndex, result: InvokeResult) {
    self.0.handle_invoke_result(idx, result)
  }

  fn throughput_hint(&self) -> usize {
    self.0.throughput_hint()
  }
}
