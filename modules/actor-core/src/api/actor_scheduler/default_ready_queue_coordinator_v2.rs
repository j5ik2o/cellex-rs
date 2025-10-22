//! DefaultReadyQueueCoordinatorV2 - Default implementation with &self methods
//!
//! This module provides a Mutex-based implementation that uses interior mutability
//! to eliminate external Mutex requirements.
//!
//! # Key Improvements over V1
//!
//! - **V1**: Requires `&mut self` → Can't be shared without external Mutex
//! - **V2**: Uses `&self` with internal Mutex → Can be wrapped in Arc directly
//!
//! # Performance Characteristics
//!
//! - **register_ready**: O(1) with Mutex lock
//! - **drain_ready_cycle**: O(batch_size) with Mutex lock
//! - **Scalability**: Sequential access due to Mutex (optimal for 1-4 threads)
//!
//! # Comparison with LockFreeCoordinatorV2
//!
//! - **DefaultReadyQueueCoordinatorV2**: Simpler, lower latency for low concurrency
//! - **LockFreeCoordinatorV2**: Better scalability for high concurrency (5+ threads)

use core::task::{Context, Poll};
use std::{
  collections::{HashSet, VecDeque},
  sync::{Arc, Mutex},
};

use super::{InvokeResult, MailboxIndex, ReadyQueueCoordinatorV2};

/// Internal state for the coordinator
struct CoordinatorState {
  queue:          VecDeque<MailboxIndex>,
  queued:         HashSet<MailboxIndex>,
  signal_pending: bool,
}

/// Default coordinator with interior mutability (V2)
///
/// This implementation uses Mutex for thread-safe access:
/// - `Arc<Mutex<CoordinatorState>>` for protected state
/// - All methods use `&self` for shared access
///
/// # Concurrency Model
///
/// All operations acquire the Mutex:
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use std::sync::Arc;
/// # use cellex_actor_core_rs::api::actor_scheduler::{
/// #   DefaultReadyQueueCoordinatorV2, ReadyQueueCoordinatorV2, MailboxIndex
/// # };
/// let coord = Arc::new(DefaultReadyQueueCoordinatorV2::new(32));
///
/// // Thread 1
/// coord.register_ready(MailboxIndex::new(0, 0)); // Acquires Mutex
///
/// // Thread 2
/// coord.register_ready(MailboxIndex::new(1, 0)); // Waits for Mutex
/// # }
/// ```
///
/// # Performance Characteristics
///
/// - **register_ready**: O(1) with Mutex lock (~30-50ns)
/// - **drain_ready_cycle**: O(batch_size) with Mutex lock
/// - **Scalability**: Sequential due to Mutex (best for 1-4 threads)
pub struct DefaultReadyQueueCoordinatorV2 {
  /// Mutex-protected state
  state: Arc<Mutex<CoordinatorState>>,

  /// Throughput hint (immutable)
  throughput: usize,
}

impl DefaultReadyQueueCoordinatorV2 {
  /// Create a new coordinator
  ///
  /// # Arguments
  ///
  /// * `throughput` - Messages per invocation hint
  ///
  /// # Examples
  ///
  /// ```rust
  /// # #[cfg(feature = "std")] {
  /// # use std::sync::Arc;
  /// # use cellex_actor_core_rs::api::actor_scheduler::DefaultReadyQueueCoordinatorV2;
  /// let coord = Arc::new(DefaultReadyQueueCoordinatorV2::new(32));
  /// // Can be shared across threads without additional Mutex!
  /// # }
  /// ```
  pub fn new(throughput: usize) -> Self {
    Self {
      state: Arc::new(Mutex::new(CoordinatorState {
        queue:          VecDeque::new(),
        queued:         HashSet::new(),
        signal_pending: false,
      })),
      throughput,
    }
  }

  /// Wait for signal asynchronously (std feature only)
  ///
  /// # Examples
  ///
  /// ```rust
  /// # #[cfg(feature = "std")] {
  /// # use std::sync::Arc;
  /// # use cellex_actor_core_rs::api::actor_scheduler::DefaultReadyQueueCoordinatorV2;
  /// # async fn example() {
  /// let coord = Arc::new(DefaultReadyQueueCoordinatorV2::new(32));
  /// coord.wait_for_signal().await;
  /// # }
  /// # }
  /// ```
  pub async fn wait_for_signal(&self) {
    use std::future::poll_fn;
    poll_fn(|cx| {
      let mut state = self.state.lock().unwrap();
      if state.signal_pending {
        state.signal_pending = false;
        Poll::Ready(())
      } else {
        // In real implementation, we would register waker
        cx.waker().wake_by_ref();
        Poll::Pending
      }
    })
    .await
  }
}

impl ReadyQueueCoordinatorV2 for DefaultReadyQueueCoordinatorV2 {
  fn register_ready(&self, idx: MailboxIndex) {
    let mut state = self.state.lock().unwrap();
    if state.queued.insert(idx) {
      state.queue.push_back(idx);
      state.signal_pending = true;
    }
  }

  fn unregister(&self, idx: MailboxIndex) {
    let mut state = self.state.lock().unwrap();
    state.queued.remove(&idx);
    // Note: We don't remove from queue itself for simplicity
    // The drain operation will skip indices not in `queued`
  }

  fn drain_ready_cycle(&self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
    let mut state = self.state.lock().unwrap();
    out.clear();

    for _ in 0..max_batch {
      if let Some(idx) = state.queue.pop_front() {
        if state.queued.contains(&idx) {
          out.push(idx);
          state.queued.remove(&idx);
        }
      } else {
        break;
      }
    }
  }

  fn poll_wait_signal(&self, _cx: &mut Context<'_>) -> Poll<()> {
    let mut state = self.state.lock().unwrap();
    if state.signal_pending {
      state.signal_pending = false;
      Poll::Ready(())
    } else {
      Poll::Pending
    }
  }

  fn handle_invoke_result(&self, idx: MailboxIndex, result: InvokeResult) {
    match result {
      | InvokeResult::Completed { ready_hint: true } => {
        // Re-register for next cycle
        self.register_ready(idx);
      },
      | InvokeResult::Completed { ready_hint: false } | InvokeResult::Suspended { .. } | InvokeResult::Stopped => {
        // Unregister the mailbox
        self.unregister(idx);
      },
      | InvokeResult::Yielded => {
        // Re-register for next cycle
        self.register_ready(idx);
      },
      | InvokeResult::Failed { retry_after, .. } => {
        if retry_after.is_some() {
          // In real implementation, schedule delayed re-register
          self.unregister(idx);
        } else {
          // Permanent failure
          self.unregister(idx);
        }
      },
    }
  }

  fn throughput_hint(&self) -> usize {
    self.throughput
  }
}

// Safety: All internal state uses thread-safe types
// - Arc<Mutex<CoordinatorState>>: thread-safe
unsafe impl Send for DefaultReadyQueueCoordinatorV2 {}
unsafe impl Sync for DefaultReadyQueueCoordinatorV2 {}

#[cfg(test)]
mod tests;
