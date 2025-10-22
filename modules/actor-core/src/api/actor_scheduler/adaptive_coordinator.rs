//! AdaptiveCoordinator - Adaptive ReadyQueueCoordinator implementation
//!
//! This module provides a coordinator that automatically selects the best implementation
//! based on the concurrency hint provided at construction time.
//!
//! # Selection Strategy
//!
//! - **1-4 threads**: Use DefaultReadyQueueCoordinator (Mutex-based)
//!   - Simpler implementation
//!   - Lower latency (~30ns register_ready)
//!   - Lower memory overhead
//!
//! - **5+ threads**: Use LockFreeCoordinator (DashSet/SegQueue-based)
//!   - Better scalability
//!   - Reduced lock contention
//!   - Higher throughput under high concurrency

use core::task::{Context, Poll};

use super::{InvokeResult, MailboxIndex, ReadyQueueCoordinator};

#[cfg(feature = "std")]
use super::DefaultReadyQueueCoordinator;
#[cfg(feature = "new-scheduler")]
use super::LockFreeCoordinator;

#[cfg(test)]
mod tests;

/// Adaptive coordinator that selects the optimal implementation
/// based on concurrency characteristics.
///
/// # Examples
///
/// ```rust
/// # #[cfg(all(feature = "std", feature = "new-scheduler"))] {
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
  #[cfg(feature = "std")]
  Locked(DefaultReadyQueueCoordinator),

  /// Lock-free implementation (optimal for 5+ threads)
  #[cfg(feature = "new-scheduler")]
  LockFree(LockFreeCoordinator),
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
  /// - `std` feature: Required for DefaultReadyQueueCoordinator
  /// - `new-scheduler` feature: Required for LockFreeCoordinator
  ///
  /// # Example
  ///
  /// ```rust
  /// # #[cfg(all(feature = "std", feature = "new-scheduler"))] {
  /// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
  /// let coordinator = AdaptiveCoordinator::new(32, 4);
  /// # }
  /// ```
  pub fn new(throughput: usize, concurrency_hint: usize) -> Self {
    #[cfg(all(feature = "new-scheduler", feature = "std"))]
    {
      if concurrency_hint <= 4 {
        Self::Locked(DefaultReadyQueueCoordinator::new(throughput))
      } else {
        Self::LockFree(LockFreeCoordinator::new(throughput))
      }
    }

    #[cfg(all(not(feature = "new-scheduler"), feature = "std"))]
    {
      let _ = concurrency_hint; // Suppress unused warning
      Self::Locked(DefaultReadyQueueCoordinator::new(throughput))
    }

    #[cfg(all(feature = "new-scheduler", not(feature = "std")))]
    {
      let _ = concurrency_hint; // Suppress unused warning
      Self::LockFree(LockFreeCoordinator::new(throughput))
    }

    #[cfg(all(not(feature = "new-scheduler"), not(feature = "std")))]
    {
      compile_error!("AdaptiveCoordinator requires either 'std' or 'new-scheduler' feature");
    }
  }

  /// Create a coordinator with explicit implementation selection
  ///
  /// # Arguments
  ///
  /// * `throughput` - Messages per invocation hint
  /// * `use_lockfree` - If true, use LockFreeCoordinator; otherwise use DefaultReadyQueueCoordinator
  ///
  /// # Panics
  ///
  /// Panics if the requested implementation is not available (feature not enabled)
  ///
  /// # Example
  ///
  /// ```rust
  /// # #[cfg(all(feature = "std", feature = "new-scheduler"))] {
  /// # use cellex_actor_core_rs::api::actor_scheduler::AdaptiveCoordinator;
  /// // Force lock-free implementation
  /// let coordinator = AdaptiveCoordinator::with_strategy(32, true);
  /// # }
  /// ```
  pub fn with_strategy(throughput: usize, use_lockfree: bool) -> Self {
    #[cfg(all(feature = "new-scheduler", feature = "std"))]
    {
      if use_lockfree {
        Self::LockFree(LockFreeCoordinator::new(throughput))
      } else {
        Self::Locked(DefaultReadyQueueCoordinator::new(throughput))
      }
    }

    #[cfg(all(not(feature = "new-scheduler"), feature = "std"))]
    {
      if use_lockfree {
        panic!("LockFreeCoordinator requires 'new-scheduler' feature");
      }
      Self::Locked(DefaultReadyQueueCoordinator::new(throughput))
    }

    #[cfg(all(feature = "new-scheduler", not(feature = "std")))]
    {
      if !use_lockfree {
        panic!("DefaultReadyQueueCoordinator requires 'std' feature");
      }
      Self::LockFree(LockFreeCoordinator::new(throughput))
    }

    #[cfg(all(not(feature = "new-scheduler"), not(feature = "std")))]
    {
      compile_error!("AdaptiveCoordinator requires either 'std' or 'new-scheduler' feature");
    }
  }
}

impl ReadyQueueCoordinator for AdaptiveCoordinator {
  fn register_ready(&mut self, idx: MailboxIndex) {
    match self {
      #[cfg(feature = "std")]
      Self::Locked(coord) => coord.register_ready(idx),
      #[cfg(feature = "new-scheduler")]
      Self::LockFree(coord) => coord.register_ready(idx),
    }
  }

  fn unregister(&mut self, idx: MailboxIndex) {
    match self {
      #[cfg(feature = "std")]
      Self::Locked(coord) => coord.unregister(idx),
      #[cfg(feature = "new-scheduler")]
      Self::LockFree(coord) => coord.unregister(idx),
    }
  }

  fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
    match self {
      #[cfg(feature = "std")]
      Self::Locked(coord) => coord.drain_ready_cycle(max_batch, out),
      #[cfg(feature = "new-scheduler")]
      Self::LockFree(coord) => coord.drain_ready_cycle(max_batch, out),
    }
  }

  fn poll_wait_signal(&mut self, cx: &mut Context<'_>) -> Poll<()> {
    match self {
      #[cfg(feature = "std")]
      Self::Locked(coord) => coord.poll_wait_signal(cx),
      #[cfg(feature = "new-scheduler")]
      Self::LockFree(coord) => coord.poll_wait_signal(cx),
    }
  }

  fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult) {
    match self {
      #[cfg(feature = "std")]
      Self::Locked(coord) => coord.handle_invoke_result(idx, result),
      #[cfg(feature = "new-scheduler")]
      Self::LockFree(coord) => coord.handle_invoke_result(idx, result),
    }
  }

  fn throughput_hint(&self) -> usize {
    match self {
      #[cfg(feature = "std")]
      Self::Locked(coord) => coord.throughput_hint(),
      #[cfg(feature = "new-scheduler")]
      Self::LockFree(coord) => coord.throughput_hint(),
    }
  }
}
