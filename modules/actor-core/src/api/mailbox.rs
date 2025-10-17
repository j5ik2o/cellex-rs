//! Public mailbox abstractions shared across the crate.

use crate::internal::metrics::MetricsSinkShared;
use crate::internal::scheduler::ReadyQueueHandle;
use cellex_utils_core_rs::{QueueError, QueueSize};
use core::future::Future;

/// Mailbox concurrency modes
mod mailbox_concurrency;
/// Mailbox runtime abstraction
mod mailbox_factory;
/// Mailbox handle trait
mod mailbox_handle;
/// Mailbox configuration options
mod mailbox_options;
/// Mailbox factory trait
mod mailbox_producer;
/// Mailbox signaling mechanisms
mod mailbox_signal;
mod messages;
/// Queue-based mailbox implementation
mod queue_mailbox;
/// Queue mailbox factory
mod queue_mailbox_producer;
/// Single-threaded mailbox
mod single_thread;
/// Thread-safe mailbox
mod thread_safe;

pub use mailbox_concurrency::*;
pub use mailbox_factory::*;
pub use mailbox_handle::*;
pub use mailbox_options::*;
pub use mailbox_producer::*;
pub use mailbox_signal::*;
pub use messages::*;
pub use queue_mailbox::*;
pub use queue_mailbox_producer::*;
pub use single_thread::*;
pub use thread_safe::*;

/// Type alias for mailbox and producer pair.
///
/// Pair of receiver and sender handles returned when creating a mailbox.
pub type MailboxPair<H, P> = (H, P);

/// Mailbox abstraction that decouples message queue implementations from core logic.
///
/// Abstraction trait that decouples message queue implementations from core logic.
/// Enables unified handling of various queue implementations (bounded/unbounded, prioritized, etc.).
///
/// # Type Parameters
/// - `M`: Type of the message to process
pub trait Mailbox<M> {
  /// Error type for message sending
  type SendError;

  /// Future type for message reception
  type RecvFuture<'a>: Future<Output = Result<M, QueueError<M>>> + 'a
  where
    Self: 'a;

  /// Attempts to send a message (non-blocking).
  ///
  /// # Arguments
  /// - `message`: Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, `Err(SendError)` on failure
  fn try_send(&self, message: M) -> Result<(), Self::SendError>;

  /// Receives a message asynchronously.
  ///
  /// # Returns
  /// Future for message reception
  fn recv(&self) -> Self::RecvFuture<'_>;

  /// Gets the number of messages in the mailbox.
  ///
  /// Default implementation returns unlimited.
  fn len(&self) -> QueueSize {
    QueueSize::limitless()
  }

  /// Gets the capacity of the mailbox.
  ///
  /// Default implementation returns unlimited.
  fn capacity(&self) -> QueueSize {
    QueueSize::limitless()
  }

  /// Checks if the mailbox is empty.
  ///
  /// # Returns
  /// `true` if empty, `false` if there are messages
  fn is_empty(&self) -> bool {
    self.len() == QueueSize::Limited(0)
  }

  /// Closes the mailbox.
  ///
  /// Default implementation does nothing.
  fn close(&self) {}

  /// Checks if the mailbox is closed.
  ///
  /// Default implementation always returns `false`.
  ///
  /// # Returns
  /// `true` if closed, `false` if open
  fn is_closed(&self) -> bool {
    false
  }

  /// Injects a metrics sink for enqueue instrumentation. Default: no-op.
  fn set_metrics_sink(&mut self, _sink: Option<MetricsSinkShared>) {}

  /// Installs a scheduler hook invoked on message arrivals. Default: no-op.
  fn set_scheduler_hook(&mut self, _hook: Option<ReadyQueueHandle>) {}
}
