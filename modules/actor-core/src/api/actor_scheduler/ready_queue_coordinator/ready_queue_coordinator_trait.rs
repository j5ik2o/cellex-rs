//! ReadyQueueCoordinator trait - Ready queue coordination interface

use alloc::vec::Vec;
use core::task::{Context, Poll};

use super::{InvokeResult, MailboxIndex};

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
#[allow(dead_code)]
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
