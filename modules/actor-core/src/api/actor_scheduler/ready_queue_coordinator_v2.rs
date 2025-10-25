//! ReadyQueueCoordinatorV2 - Redesigned trait with interior mutability
//!
//! This module provides a redesigned coordinator trait that eliminates the need
//! for external Mutex wrapping by using `&self` methods with interior mutability.
//!
//! # Key Differences from V1
//!
//! - **V1**: `&mut self` → Requires `Mutex` wrapper → Serializes lock-free operations
//! - **V2**: `&self` → No Mutex needed → Full lock-free capability
//!
//! # Design Principles
//!
//! 1. **Interior Mutability**: Use `Arc<DashSet>`, `Arc<SegQueue>` internally
//! 2. **Zero External Locking**: Implementations handle concurrency internally
//! 3. **Backward Compatible**: Can coexist with V1 during migration

use alloc::vec::Vec;
use core::task::{Context, Poll};

use super::{InvokeResult, MailboxIndex};

/// V2 trait for ready queue coordination with interior mutability
///
/// This trait uses `&self` methods to allow concurrent access without
/// external Mutex wrapping. Implementations must handle thread safety
/// internally using atomic operations or lock-free data structures.
///
/// # Thread Safety
///
/// All methods are safe to call concurrently from multiple threads.
/// Implementations guarantee:
/// - No data races
/// - Progress guarantee (lock-free or wait-free)
/// - Linearizability of operations
///
/// # Migration from V1
///
/// ```rust
/// # extern crate alloc;
/// # use cellex_actor_core_rs::api::actor_scheduler::{DefaultReadyQueueCoordinatorV2, ReadyQueueCoordinatorV2, MailboxIndex};
/// # use cellex_utils_core_rs::ArcShared;
/// let coord = ArcShared::new(DefaultReadyQueueCoordinatorV2::new(32));
/// let idx = MailboxIndex::new(0, 0);
/// coord.register_ready(idx);
/// ```
pub trait ReadyQueueCoordinatorV2: Send + Sync {
  /// Register a mailbox as ready for processing
  ///
  /// This method is safe to call concurrently from multiple threads.
  /// Duplicate registrations are automatically detected and ignored.
  ///
  /// # Parameters
  ///
  /// * `idx` - Mailbox index to register
  ///
  /// # Thread Safety
  ///
  /// This method uses interior mutability and is safe for concurrent calls.
  fn register_ready(&self, idx: MailboxIndex);

  /// Unregister a mailbox from the ready queue
  ///
  /// This method is safe to call concurrently from multiple threads.
  /// If the mailbox is not registered, this is a no-op.
  ///
  /// # Parameters
  ///
  /// * `idx` - Mailbox index to unregister
  ///
  /// # Thread Safety
  ///
  /// This method uses interior mutability and is safe for concurrent calls.
  fn unregister(&self, idx: MailboxIndex);

  /// Drain ready queue and fill the provided buffer
  ///
  /// This method extracts up to `max_batch` ready mailboxes from the queue.
  /// The caller owns the output buffer to avoid allocation on each call.
  ///
  /// # Parameters
  ///
  /// * `max_batch` - Maximum number of indices to drain
  /// * `out` - Output buffer to fill with ready indices (will be cleared first)
  ///
  /// # Thread Safety
  ///
  /// This method is safe to call concurrently. Multiple drainers will
  /// receive disjoint sets of mailboxes (no duplicates across threads).
  ///
  /// # Note
  ///
  /// The `out` parameter requires `&mut Vec` which is NOT shared across threads.
  /// Each thread provides its own buffer.
  fn drain_ready_cycle(&self, max_batch: usize, out: &mut Vec<MailboxIndex>);

  /// Poll for signal notification
  ///
  /// # Returns
  ///
  /// - `Poll::Ready(())` if a signal is available
  /// - `Poll::Pending` if no signal is available (caller should wait)
  ///
  /// # Thread Safety
  ///
  /// This method is safe to call concurrently from multiple threads.
  fn poll_wait_signal(&self, cx: &mut Context<'_>) -> Poll<()>;

  /// Handle the result of message invocation
  ///
  /// Based on the result, the coordinator will:
  /// - Re-register the mailbox if `ready_hint` is true
  /// - Unregister if suspended or stopped
  /// - Schedule retry if failed
  ///
  /// # Parameters
  ///
  /// * `idx` - Mailbox index that completed invocation
  /// * `result` - Result of the invocation
  ///
  /// # Thread Safety
  ///
  /// This method is safe to call concurrently from multiple threads.
  fn handle_invoke_result(&self, idx: MailboxIndex, result: InvokeResult);

  /// Get throughput hint (messages per invocation)
  ///
  /// This value is used by the invoker to limit the number of messages
  /// processed in a single invocation for fairness.
  ///
  /// # Returns
  ///
  /// Maximum number of messages to process per invocation
  fn throughput_hint(&self) -> usize;
}
