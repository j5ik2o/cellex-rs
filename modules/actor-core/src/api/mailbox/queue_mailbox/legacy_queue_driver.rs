use core::ops::Deref;

use cellex_utils_core_rs::{
  collections::queue::QueueError,
  v2::collections::queue::backend::OfferOutcome,
  Element,
  QueueRw,
  QueueSize,
};

use crate::api::metrics::MetricsSinkShared;

use super::{MailboxQueueDriver, QueuePollOutcome};

/// Adapter that allows legacy `QueueRw` implementations to satisfy `MailboxQueueDriver`.
pub struct LegacyQueueDriver<Q> {
  queue: Q,
}

impl<Q> LegacyQueueDriver<Q> {
  /// Creates a new driver wrapping the provided legacy queue instance.
  pub const fn new(queue: Q) -> Self {
    Self { queue }
  }

  /// Consumes the driver and returns the wrapped queue.
  pub fn into_inner(self) -> Q {
    self.queue
  }
}

impl<Q> From<Q> for LegacyQueueDriver<Q> {
  fn from(value: Q) -> Self {
    Self::new(value)
  }
}

impl<Q> Clone for LegacyQueueDriver<Q>
where
  Q: Clone,
{
  fn clone(&self) -> Self {
    Self { queue: self.queue.clone() }
  }
}

impl<Q> Deref for LegacyQueueDriver<Q> {
  type Target = Q;

  fn deref(&self) -> &Self::Target {
    &self.queue
  }
}

impl<M, Q> MailboxQueueDriver<M> for LegacyQueueDriver<Q>
where
  M: Element,
  Q: QueueRw<M> + Clone,
{
  fn len(&self) -> QueueSize {
    self.queue.len()
  }

  fn capacity(&self) -> QueueSize {
    self.queue.capacity()
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    match self.queue.offer(message) {
      | Ok(()) => Ok(OfferOutcome::Enqueued),
      | Err(error) => Err(error),
    }
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    let result = self.queue.poll();
    Ok(QueuePollOutcome::from_result(result))
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    self.queue.clean_up();
    Ok(None)
  }

  fn set_metrics_sink(&self, _sink: Option<MetricsSinkShared>) {
    // Legacy queues do not expose a metrics interface. Callers continue
    // to rely on MailboxQueueCore for enqueue instrumentation.
  }
}
