use core::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{MailboxQueueDriver, QueuePollOutcome, SyncMailboxQueue},
    MailboxOverflowPolicy,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_core_rs::collections::{
  queue::{
    backend::{OfferOutcome, QueueError},
    QueueSize,
  },
  Element,
};

pub struct ArcSyncQueueDriver<M, RM>
where
  M: Element, {
  inner:   SyncMailboxQueue<M>,
  _marker: PhantomData<RM>,
}

impl<M, RM> ArcSyncQueueDriver<M, RM>
where
  M: Element,
{
  pub fn from_driver(driver: SyncMailboxQueue<M>) -> Self {
    Self { inner: driver, _marker: PhantomData }
  }
}

impl<M, RM> Clone for ArcSyncQueueDriver<M, RM>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone(), _marker: PhantomData }
  }
}

impl<M, RM> Deref for ArcSyncQueueDriver<M, RM>
where
  M: Element,
{
  type Target = SyncMailboxQueue<M>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<M, RM> DerefMut for ArcSyncQueueDriver<M, RM>
where
  M: Element,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<M, RM> MailboxQueueDriver<M> for ArcSyncQueueDriver<M, RM>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    self.inner.offer(message)
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    self.inner.poll()
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    self.inner.close()
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    self.inner.overflow_policy()
  }
}
