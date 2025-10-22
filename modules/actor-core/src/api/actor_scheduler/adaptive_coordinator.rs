//! AdaptiveCoordinator - Adaptive ReadyQueueCoordinatorV2 implementation
//!
//! This module provides a coordinator that automatically selects the best implementation
//! based on the concurrency hint provided at construction time.
//!
//! # Selection Strategy
//!
//! - **1-4 threads**: Use DefaultReadyQueueCoordinatorV2 (Mutex-based)
//!   - Simpler implementation
//!   - Lower latency (~30ns register_ready)
//!   - Lower memory overhead
//!
//! - **5+ threads**: Use LockFreeCoordinatorV2 (DashSet/SegQueue-based)
//!   - Better scalability
//!   - Reduced lock contention
//!   - Higher throughput under high concurrency

use alloc::vec::Vec;
use core::task::{Context, Poll};

use super::DefaultReadyQueueCoordinatorV2;
use super::LockFreeCoordinatorV2;
use super::{InvokeResult, MailboxIndex, ReadyQueueCoordinatorV2};

#[cfg(test)]
mod tests;

/// Adaptive coordinator that selects the optimal implementation
/// based on concurrency characteristics.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
/// // Low concurrency scenario (web server with 4 cores)
/// let coordinator = AdaptiveCoordinator::new(32, 4);
/// // → Uses DefaultReadyQueueCoordinator
///
/// // High concurrency scenario (batch processor with 16 cores)
/// let coordinator = AdaptiveCoordinator::new(32, 16);
/// // → Uses LockFreeCoordinator
/// # }
/// ```
pub enum AdaptiveCoordinator {
  /// Mutex-based implementation (optimal for 1-4 threads)
  Locked(DefaultReadyQueueCoordinatorV2),

  /// Lock-free implementation (optimal for 5+ threads)
  LockFree(LockFreeCoordinatorV2),
}

impl AdaptiveCoordinator {
  /// Create a new adaptive coordinator with automatic implementation selection
  ///
  /// # Arguments
  ///
  /// * `throughput` - Messages per invocation hint
  /// * `concurrency_hint` - Expected number of concurrent worker threads
  ///
  /// # Selection Logic
  ///
  /// - `concurrency_hint <= 4` → DefaultReadyQueueCoordinator
  /// - `concurrency_hint >= 5` → LockFreeCoordinator (if available)
  ///
  /// # Feature Requirements
  ///
  /// - `std` feature: Optional for DefaultReadyQueueCoordinator
  ///
  /// # Example
  ///
  /// ```rust
  /// # #[cfg(feature = "std")] {
  /// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
  /// let coordinator = AdaptiveCoordinator::new(32, 4);
  /// # }
  /// ```
  pub fn new(throughput: usize, concurrency_hint: usize) -> Self {
    if concurrency_hint <= 4 {
      Self::Locked(DefaultReadyQueueCoordinatorV2::new(throughput))
    } else {
      Self::LockFree(LockFreeCoordinatorV2::new(throughput))
    }
  }

  /// Create a coordinator with explicit implementation selection
  ///
  /// # Arguments
  ///
  /// * `throughput` - Messages per invocation hint
  /// * `use_lockfree` - If true, use LockFreeCoordinator; otherwise use
  ///   DefaultReadyQueueCoordinator
  ///
  /// # Panics
  ///
  /// Panics if the requested implementation is not available (feature not enabled)
  ///
  /// # Example
  ///
  /// ```rust
  /// # #[cfg(feature = "std")] {
  /// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
  /// // Force lock-free implementation
  /// let coordinator = AdaptiveCoordinator::with_strategy(32, true);
  /// # }
  /// ```
  pub fn with_strategy(throughput: usize, use_lockfree: bool) -> Self {
    if use_lockfree {
      Self::LockFree(LockFreeCoordinatorV2::new(throughput))
    } else {
      Self::Locked(DefaultReadyQueueCoordinatorV2::new(throughput))
    }
  }
}

impl ReadyQueueCoordinatorV2 for AdaptiveCoordinator {
  fn register_ready(&self, idx: MailboxIndex) {
    match self {
      Self::Locked(coord) => coord.register_ready(idx),
      Self::LockFree(coord) => coord.register_ready(idx),
    }
  }

  fn unregister(&self, idx: MailboxIndex) {
    match self {
      Self::Locked(coord) => coord.unregister(idx),
      Self::LockFree(coord) => coord.unregister(idx),
    }
  }

  fn drain_ready_cycle(&self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
    match self {
      Self::Locked(coord) => coord.drain_ready_cycle(max_batch, out),
      Self::LockFree(coord) => coord.drain_ready_cycle(max_batch, out),
    }
  }

  fn poll_wait_signal(&self, cx: &mut Context<'_>) -> Poll<()> {
    match self {
      Self::Locked(coord) => coord.poll_wait_signal(cx),
      Self::LockFree(coord) => coord.poll_wait_signal(cx),
    }
  }

  fn handle_invoke_result(&self, idx: MailboxIndex, result: InvokeResult) {
    match self {
      Self::Locked(coord) => coord.handle_invoke_result(idx, result),
      Self::LockFree(coord) => coord.handle_invoke_result(idx, result),
    }
  }

  fn throughput_hint(&self) -> usize {
    match self {
      Self::Locked(coord) => coord.throughput_hint(),
      Self::LockFree(coord) => coord.throughput_hint(),
    }
  }
}
