//! Public mailbox abstractions shared across the crate.

use core::future::Future;

use cellex_utils_core_rs::{collections::queue::QueueError, QueueSize};

use crate::api::metrics::MetricsSinkShared;

/// Mailbox concurrency modes
mod mailbox_concurrency;
pub mod messages;
/// Queue-based mailbox implementation
pub mod queue_mailbox;
/// Queue mailbox producer utilities shared across runtimes.
mod queue_mailbox_producer;
/// Single-threaded mailbox
mod single_thread;
/// Thread-safe mailbox
mod thread_safe;

pub use mailbox_concurrency::*;
pub use queue_mailbox_producer::*;
pub use single_thread::*;
pub use thread_safe::*;

use crate::api::actor_scheduler::ready_queue_scheduler::ReadyQueueHandle;
// Re-export shared mailbox types
pub use crate::shared::mailbox::{
  factory::{MailboxFactory, MailboxPair},
  handle::MailboxHandle,
  options::MailboxOptions,
  producer::MailboxProducer,
  signal::MailboxSignal,
};

/// Mailbox abstraction that decouples message queue implementations from core logic.
///
/// Abstraction trait that decouples message queue implementations from core logic.
/// Enables unified handling of various queue implementations (bounded/unbounded, prioritized,
/// etc.).
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
  ///
  /// # Errors
  /// Returns [`Self::SendError`] when the mailbox cannot enqueue the message.
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
