use cellex_utils_core_rs::{collections::queue::QueueError, Element, QueueRw, SharedBound};

use crate::api::{
  actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
  mailbox::{queue_mailbox::QueueMailboxInternal, MailboxSignal},
  metrics::MetricsSinkShared,
};

/// Sending handle that shares ownership with the mailbox.
#[derive(Clone)]
pub struct QueueMailboxProducer<Q, S> {
  inner: QueueMailboxInternal<Q, S>,
}

impl<Q, S> core::fmt::Debug for QueueMailboxProducer<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailboxProducer").finish()
  }
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<Q, S> Send for QueueMailboxProducer<Q, S>
where
  Q: SharedBound,
  S: SharedBound,
{
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<Q, S> Sync for QueueMailboxProducer<Q, S>
where
  Q: SharedBound,
  S: SharedBound,
{
}

impl<Q, S> QueueMailboxProducer<Q, S> {
  pub(crate) fn from_internal(inner: QueueMailboxInternal<Q, S>) -> Self {
    Self { inner }
  }

  /// Attempts to send a message without blocking.
  pub fn try_send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.inner.try_send(message)
  }

  /// Convenience method delegating to [`Self::try_send`].
  pub fn send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send(message)
  }

  /// Returns a reference to the underlying queue.
  #[must_use]
  pub fn queue(&self) -> &Q {
    self.inner.queue()
  }

  /// Updates the metrics sink observed by this producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }

  /// Updates the scheduler hook observed by this producer.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.inner.set_scheduler_hook(hook);
  }
}
