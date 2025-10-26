use core::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use cellex_actor_core_rs::{
  api::{
    mailbox::{
      queue_mailbox::{MailboxQueueDriver, QueuePollOutcome},
      MailboxOverflowPolicy,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_core_rs::{
  collections::queue::QueueError, v2::collections::queue::backend::OfferOutcome, Element, QueueSize,
};

use super::priority_sync_driver::PrioritySyncQueueDriver;

/// Wrapper that associates a `PrioritySyncQueueDriver` with the raw mutex parameter used by
/// embedded mailbox signals.
pub struct ArcPrioritySyncQueueDriver<M, RM>
where
  M: Element, {
  inner:   PrioritySyncQueueDriver<M>,
  _marker: PhantomData<RM>,
}

impl<M, RM> ArcPrioritySyncQueueDriver<M, RM>
where
  M: Element,
{
  pub fn from_driver(driver: PrioritySyncQueueDriver<M>) -> Self {
    Self { inner: driver, _marker: PhantomData }
  }
}

impl<M, RM> Clone for ArcPrioritySyncQueueDriver<M, RM>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone(), _marker: PhantomData }
  }
}

impl<M, RM> Deref for ArcPrioritySyncQueueDriver<M, RM>
where
  M: Element,
{
  type Target = PrioritySyncQueueDriver<M>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<M, RM> DerefMut for ArcPrioritySyncQueueDriver<M, RM>
where
  M: Element,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<M, RM> MailboxQueueDriver<PriorityEnvelope<M>> for ArcPrioritySyncQueueDriver<M, RM>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }

  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<OfferOutcome, QueueError<PriorityEnvelope<M>>> {
    self.inner.offer(envelope)
  }

  fn poll(&self) -> Result<QueuePollOutcome<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.inner.poll()
  }

  fn close(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.inner.close()
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    self.inner.overflow_policy()
  }
}
