use cellex_utils_core_rs::collections::{
  queue::{
    backend::{OfferOutcome, QueueError},
    QueueSize,
  },
  Element,
};

use super::{MailboxQueue, MailboxQueueBackend, QueuePollOutcome};
use crate::api::metrics::MetricsSinkShared;

/// Trait describing the behavior required from a system-only mailbox lane.
pub trait SystemMailboxLane<M>: MailboxQueue<M>
where
  M: Element, {
  /// Returns `true` when the given message should be stored in the system lane.
  fn accepts(&self, message: &M) -> bool;
}

impl<M> SystemMailboxLane<M> for ()
where
  M: Element,
{
  fn accepts(&self, _message: &M) -> bool {
    false
  }
}

impl<M> MailboxQueueBackend<M> for ()
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    QueueSize::limited(0)
  }

  fn capacity(&self) -> QueueSize {
    QueueSize::limited(0)
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    Err(QueueError::OfferError(message))
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    Ok(QueuePollOutcome::Empty)
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    Ok(None)
  }

  fn set_metrics_sink(&self, _sink: Option<MetricsSinkShared>) {}
}
