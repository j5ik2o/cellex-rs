use cellex_utils_core_rs::{
  collections::queue::QueueError,
  v2::collections::queue::backend::OfferOutcome,
  Element,
  QueueSize,
};

use crate::api::metrics::MetricsSinkShared;
use super::QueuePollOutcome;

/// Abstraction over queue backends used by `QueueMailbox`.
pub trait MailboxQueueDriver<M>: Clone
where
  M: Element,
{
  /// Returns the current queue length in abstract `QueueSize` units.
  fn len(&self) -> QueueSize;

  /// Returns the capacity exposed by the underlying queue backend.
  fn capacity(&self) -> QueueSize;

  /// Attempts to enqueue a message and reports whether additional side effects occurred.
  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>>;

  /// Polls the queue for the next message or outcome.
  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>>;

  /// Closes the queue, returning the preserved message if the backend exposes one.
  fn close(&self) -> Result<Option<M>, QueueError<M>>;

  /// Installs or removes a metrics sink used for queue-level instrumentation.
  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>);
}
