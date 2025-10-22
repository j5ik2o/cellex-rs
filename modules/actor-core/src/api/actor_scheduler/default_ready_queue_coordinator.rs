//! DefaultReadyQueueCoordinator - Default implementation of ReadyQueueCoordinator
//!
//! This module provides the default implementation using std collections.
//! This is a Phase 0 prototype implementation.

use super::{InvokeResult, MailboxIndex, ReadyQueueCoordinator};
use core::task::{Context, Poll};
use std::{
  collections::{HashSet, VecDeque},
  sync::{Arc, Mutex},
};

/// Internal state for the coordinator
struct CoordinatorState {
  queue:          VecDeque<MailboxIndex>,
  queued:         HashSet<MailboxIndex>,
  signal_pending: bool,
}

/// Default implementation of ReadyQueueCoordinator
///
/// This is a simple prototype implementation for Phase 0.
/// It uses a Mutex-protected queue and signal channel.
///
/// # Future Improvements
///
/// - Use DashSet for lock-free duplicate detection
/// - Use MPSC channel for signal notification
/// - Minimize critical section duration
pub struct DefaultReadyQueueCoordinator {
  state:      Arc<Mutex<CoordinatorState>>,
  throughput: usize,
}

impl DefaultReadyQueueCoordinator {
  /// Create a new DefaultReadyQueueCoordinator
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
  /// This is a helper method for std environments.
  /// In no_std, use `poll_wait_signal` directly.
  pub async fn wait_for_signal(&self) {
    use std::future::poll_fn;
    poll_fn(|cx| {
      let mut state = self.state.lock().unwrap();
      if state.signal_pending {
        state.signal_pending = false;
        Poll::Ready(())
      } else {
        // In real implementation, we would register waker
        // For prototype, just return Pending
        cx.waker().wake_by_ref();
        Poll::Pending
      }
    })
    .await
  }
}

impl ReadyQueueCoordinator for DefaultReadyQueueCoordinator {
  fn register_ready(&mut self, idx: MailboxIndex) {
    let mut state = self.state.lock().unwrap();
    if state.queued.insert(idx) {
      state.queue.push_back(idx);
      state.signal_pending = true;
    }
  }

  fn unregister(&mut self, idx: MailboxIndex) {
    let mut state = self.state.lock().unwrap();
    state.queued.remove(&idx);
    // Note: We don't remove from queue itself for simplicity
    // The drain operation will skip indices not in `queued`
  }

  fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
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

  fn poll_wait_signal(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
    let mut state = self.state.lock().unwrap();
    if state.signal_pending {
      state.signal_pending = false;
      Poll::Ready(())
    } else {
      Poll::Pending
    }
  }

  fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult) {
    match result {
      | InvokeResult::Completed { ready_hint: true } => {
        self.register_ready(idx);
      },
      | InvokeResult::Completed { ready_hint: false } | InvokeResult::Suspended { .. } | InvokeResult::Stopped => {
        self.unregister(idx);
      },
      | InvokeResult::Yielded => {
        // Re-register for next cycle
        self.register_ready(idx);
      },
      | InvokeResult::Failed { retry_after, .. } => {
        if retry_after.is_some() {
          // In real implementation, schedule delayed re-register
          // For prototype, just unregister
          self.unregister(idx);
        } else {
          self.unregister(idx);
        }
      },
    }
  }

  fn throughput_hint(&self) -> usize {
    self.throughput
  }
}
