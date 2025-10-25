use cellex_utils_core_rs::{collections::queue::QueueError, Element};

use crate::api::metrics::MetricsSinkShared;

pub trait MailboxQueueDriver<M>: Clone
where
  M: Element,
{
  type OfferSummary;
  type PollSummary;

  fn len(&self) -> usize;

  fn capacity(&self) -> usize;

  fn offer(&self, message: M) -> Result<Self::OfferSummary, QueueError<M>>;

  fn poll(&self) -> Result<Self::PollSummary, QueueError<M>>;

  fn close(&self) -> Result<Option<M>, QueueError<M>>;

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>);
}
