//! Mailbox-specific error types and conversions from queue errors.

use cellex_utils_core_rs::{collections::queue::QueueError, Element};

use super::mailbox_overflow_policy::MailboxOverflowPolicy;

/// Unified error type surfaced by mailbox operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailboxError<M>
where
  M: Element, {
  /// The mailbox rejected a message due to capacity limits.
  QueueFull {
    /// Overflow handling strategy that produced the rejection.
    policy:    MailboxOverflowPolicy,
    /// Message that could not be enqueued.
    preserved: M,
  },
  /// The underlying queue has been disconnected permanently.
  Disconnected,
  /// The mailbox was closed. `last` carries the preserved message if available.
  Closed {
    /// Optional message retained by the queue during closure.
    last: Option<M>,
  },
  /// The operation would block because capacity is temporarily unavailable.
  Backpressure,
  /// Resource exhaustion prevented the queue from allocating storage.
  ResourceExhausted {
    /// Message associated with the allocation failure.
    preserved: M,
  },
  /// Unclassified internal error surfaced by the queue backend.
  Internal {
    /// Message associated with the failure when provided by the backend.
    preserved: M,
  },
}

impl<M> MailboxError<M>
where
  M: Element,
{
  /// Converts a `QueueError` into a `MailboxError` using a default overflow policy hint.
  pub fn from_queue_error(error: QueueError<M>) -> Self {
    match error {
      | QueueError::Full(message) => {
        Self::QueueFull { policy: MailboxOverflowPolicy::DropNewest, preserved: message }
      },
      | QueueError::OfferError(message) => Self::Internal { preserved: message },
      | QueueError::Closed(message) => Self::Closed { last: Some(message) },
      | QueueError::Disconnected => Self::Disconnected,
      | QueueError::WouldBlock => Self::Backpressure,
      | QueueError::AllocError(message) => Self::ResourceExhausted { preserved: message },
      | QueueError::Empty => Self::Internal { preserved: panic_empty_to_internal() },
    }
  }

  /// Converts a `QueueError` into a `MailboxError` with an explicit overflow policy hint.
  pub fn from_queue_error_with_policy(error: QueueError<M>, policy: MailboxOverflowPolicy) -> Self {
    match error {
      | QueueError::Full(message) => Self::QueueFull { policy, preserved: message },
      | other => Self::from_queue_error(other),
    }
  }

  /// Returns `true` when the error indicates that the mailbox transitioned into the closed state.
  #[must_use]
  pub const fn closes_mailbox(&self) -> bool {
    matches!(self, Self::Disconnected | Self::Closed { .. })
  }
}

impl<M> From<MailboxError<M>> for QueueError<M>
where
  M: Element,
{
  fn from(error: MailboxError<M>) -> Self {
    match error {
      | MailboxError::QueueFull { preserved, .. } => QueueError::Full(preserved),
      | MailboxError::Disconnected => QueueError::Disconnected,
      | MailboxError::Closed { last: Some(message) } => QueueError::Closed(message),
      | MailboxError::Closed { last: None } => QueueError::Disconnected,
      | MailboxError::Backpressure => QueueError::WouldBlock,
      | MailboxError::ResourceExhausted { preserved } => QueueError::AllocError(preserved),
      | MailboxError::Internal { preserved } => QueueError::OfferError(preserved),
    }
  }
}

fn panic_empty_to_internal<M>() -> M
where
  M: Element, {
  panic!("QueueError::Empty cannot be converted into a MailboxError; caller must treat it as a non-error outcome");
}
