//! ReadyQueueCoordinator - Ready queue coordination and signal management
//!
//! This module provides the prototype implementation of ReadyQueueCoordinator
//! as part of Phase 0 of the ActorScheduler refactoring.
//!
//! # Design Goals
//!
//! - Separate ready queue management from scheduler frontend
//! - Provide clear API for register/unregister/drain operations
//! - Enable runtime-agnostic signal handling via poll_wait_signal
//!
//! # References
//!
//! - Design doc: `docs/design/actor_scheduler_refactor.md` Section 4.4
//! - ADR: `docs/adr/2025-10-22-phase0-naming-policy.md`

use core::{
  num::NonZeroUsize,
  task::{Context, Poll},
  time::Duration,
};

#[cfg(test)]
mod tests;

// ============================================================================
// Core Types
// ============================================================================

/// Mailbox index with slot and generation for safe reuse
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MailboxIndex {
  /// Slot number in the registry
  pub slot:       u32,
  /// Generation number to prevent use-after-free
  pub generation: u32,
}

impl MailboxIndex {
  /// Create a new MailboxIndex
  pub fn new(slot: u32, generation: u32) -> Self {
    Self { slot, generation }
  }
}

/// Result of message invocation
#[derive(Debug, Clone, PartialEq)]
pub enum InvokeResult {
  /// Processing completed
  Completed {
    /// Hint: should this mailbox be re-registered to ready queue?
    ready_hint: bool,
  },
  /// Yielded for fairness (reached throughput limit)
  Yielded,
  /// Actor is suspended
  Suspended {
    /// Reason for suspension
    reason:    SuspendReason,
    /// Condition for resuming the actor
    resume_on: ResumeCondition,
  },
  /// Actor failed during processing
  Failed {
    /// Error message (simplified for prototype)
    error:       String,
    /// Optional retry delay
    retry_after: Option<Duration>,
  },
  /// Actor has stopped
  Stopped,
}

/// Reason for actor suspension
#[derive(Debug, Clone, PartialEq)]
pub enum SuspendReason {
  /// Suspended due to backpressure
  Backpressure,
  /// Suspended while awaiting external event
  AwaitExternal,
  /// Suspended due to rate limiting
  RateLimit,
  /// User-defined suspension reason
  UserDefined,
}

/// Condition for resuming a suspended actor
#[derive(Debug, Clone, PartialEq)]
pub enum ResumeCondition {
  /// Resume when external signal is received
  ExternalSignal(SignalKey),
  /// Resume after specified duration
  After(Duration),
  /// Resume when capacity becomes available
  WhenCapacityAvailable,
}

/// Signal key for external wake-up
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalKey(pub u64);

/// Actor execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorState {
  /// Actor is actively processing messages
  Running,
  /// Actor is suspended
  Suspended,
  /// Actor is in the process of stopping
  Stopping,
  /// Actor has stopped
  Stopped,
}

/// Overflow strategy for mailbox capacity limits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowStrategy {
  /// Drop the oldest message when full
  DropOldest,
  /// Drop the newest message when full
  DropNewest,
  /// Block the producer until space is available
  BlockProducer,
  /// Reject the message immediately
  Reject,
  /// Send to dead letter queue
  DeadLetter,
}

/// Mailbox configuration options
#[derive(Debug, Clone)]
pub struct MailboxOptions {
  /// Maximum capacity (must be non-zero)
  pub capacity:           NonZeroUsize,
  /// Strategy when capacity is exceeded
  pub overflow:           OverflowStrategy,
  /// Reserved slots for system messages
  pub reserve_for_system: usize,
}

impl Default for MailboxOptions {
  fn default() -> Self {
    Self {
      capacity:           NonZeroUsize::new(1000).unwrap(),
      overflow:           OverflowStrategy::DropOldest,
      reserve_for_system: 10,
    }
  }
}

// ============================================================================
// ReadyQueueCoordinator Trait
// ============================================================================

/// Trait for ready queue coordination
///
/// This trait abstracts the ready queue management, providing methods for:
/// - Registering and unregistering mailboxes
/// - Draining ready candidates for processing
/// - Polling for signal notifications
/// - Handling invoke results
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow concurrent access
/// from multiple worker threads.
pub trait ReadyQueueCoordinator: Send + Sync {
  /// Register a mailbox as ready for processing
  fn register_ready(&mut self, idx: MailboxIndex);

  /// Unregister a mailbox from the ready queue
  fn unregister(&mut self, idx: MailboxIndex);

  /// Drain ready queue and fill the provided buffer
  ///
  /// # Arguments
  ///
  /// * `max_batch` - Maximum number of indices to drain
  /// * `out` - Output buffer to fill with ready indices
  ///
  /// # Note
  ///
  /// The caller owns the buffer to avoid allocation on each call.
  fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut Vec<MailboxIndex>);

  /// Poll for signal notification
  ///
  /// # Returns
  ///
  /// - `Poll::Ready(())` if a signal is available
  /// - `Poll::Pending` if no signal is available (caller should wait)
  fn poll_wait_signal(&mut self, cx: &mut Context<'_>) -> Poll<()>;

  /// Handle the result of message invocation
  ///
  /// Based on the result, the coordinator will:
  /// - Re-register the mailbox if `ready_hint` is true
  /// - Unregister if suspended or stopped
  /// - Schedule retry if failed
  fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult);

  /// Get throughput hint (messages per invocation)
  ///
  /// This value is used by the invoker to limit the number of messages
  /// processed in a single invocation for fairness.
  fn throughput_hint(&self) -> usize;
}

// ============================================================================
// Prototype Implementation
// ============================================================================

#[cfg(feature = "std")]
use std::collections::{HashSet, VecDeque};
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
struct CoordinatorState {
  queue:          VecDeque<MailboxIndex>,
  queued:         HashSet<MailboxIndex>,
  signal_pending: bool,
}

#[cfg(feature = "std")]
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
  #[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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
