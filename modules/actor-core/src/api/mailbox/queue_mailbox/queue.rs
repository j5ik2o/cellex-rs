use cellex_utils_core_rs::collections::{
  queue::{
    backend::{OfferOutcome, QueueError},
    QueueSize,
  },
  Element,
};

use super::{backend::MailboxQueueBackend, QueuePollOutcome};
use crate::api::{mailbox::MailboxOverflowPolicy, metrics::MetricsSinkShared};

/// Abstraction over queue types consumed by [`QueueMailboxCore`](super::QueueMailboxCore).
pub trait QueueMailboxQueue<M>: Clone
where
  M: Element, {
  /// Returns the number of messages currently stored in the queue.
  fn len(&self) -> QueueSize;

  /// Returns the maximum capacity of the queue.
  fn capacity(&self) -> QueueSize;

  /// Attempts to push a message into the queue.
  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>>;

  /// Polls the queue for the next message or status transition.
  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>>;

  /// Closes the queue, returning the preserved message if available.
  fn close(&self) -> Result<Option<M>, QueueError<M>>;

  /// Installs a metrics sink used for queue-level instrumentation.
  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>);

  /// Returns the overflow policy associated with this queue, if any.
  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    None
  }
}

impl<M, Q> QueueMailboxQueue<M> for Q
where
  M: Element,
  Q: MailboxQueueBackend<M>,
{
  fn len(&self) -> QueueSize {
    MailboxQueueBackend::len(self)
  }

  fn capacity(&self) -> QueueSize {
    MailboxQueueBackend::capacity(self)
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    MailboxQueueBackend::offer(self, message)
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    MailboxQueueBackend::poll(self)
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    MailboxQueueBackend::close(self)
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    MailboxQueueBackend::set_metrics_sink(self, sink);
  }

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    MailboxQueueBackend::overflow_policy(self)
  }
}
