use cellex_utils_core_rs::{collections::queue::QueueError, Element, QueueRw, QueueSize};

use super::{internal::QueueMailboxInternal, recv::QueueMailboxRecv};
use crate::api::{
  actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
  mailbox::{queue_mailbox_producer::QueueMailboxProducer, Mailbox, MailboxHandle, MailboxProducer, MailboxSignal},
  metrics::MetricsSinkShared,
};

/// Mailbox backed by a queue and notification signal.
pub struct QueueMailbox<Q, S> {
  inner: QueueMailboxInternal<Q, S>,
}

impl<Q, S> QueueMailbox<Q, S> {
  /// Creates a new queue mailbox.
  pub fn new(queue: Q, signal: S) -> Self {
    Self { inner: QueueMailboxInternal::new(queue, signal) }
  }

  /// Returns a reference to the underlying queue.
  #[must_use]
  pub const fn queue(&self) -> &Q {
    self.inner.queue()
  }

  /// Returns a reference to the notification signal.
  #[must_use]
  pub const fn signal(&self) -> &S {
    self.inner.signal()
  }

  pub(super) const fn inner(&self) -> &QueueMailboxInternal<Q, S> {
    &self.inner
  }

  /// Creates a producer handle that shares the queue.
  pub fn producer(&self) -> QueueMailboxProducer<Q, S>
  where
    Q: Clone,
    S: Clone, {
    QueueMailboxProducer::from_internal(self.inner.clone())
  }

  /// Injects a metrics sink used for enqueue instrumentation.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }

  /// Installs a scheduler hook that observes enqueue events.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.inner.set_scheduler_hook(hook);
  }

  /// Returns the current queue length as `usize`.
  #[must_use]
  pub fn len_usize<M>(&self) -> usize
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.inner.len::<M>().to_usize()
  }

  /// Returns the queue capacity as `usize`.
  #[must_use]
  pub fn capacity_usize<M>(&self) -> usize
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.inner.capacity::<M>().to_usize()
  }
}

impl<Q, S> Clone for QueueMailbox<Q, S>
where
  Q: Clone,
  S: Clone,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl<Q, S> core::fmt::Debug for QueueMailbox<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailbox").finish()
  }
}

impl<M, Q, S> MailboxHandle<M> for QueueMailbox<Q, S>
where
  Q: QueueRw<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  type Signal = S;

  fn signal(&self) -> Self::Signal {
    self.signal().clone()
  }

  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>> {
    self.inner.try_dequeue()
  }
}

impl<M, Q, S> MailboxProducer<M> for QueueMailboxProducer<Q, S>
where
  Q: QueueRw<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    <QueueMailboxProducer<Q, S>>::try_send(self, message)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    <QueueMailboxProducer<Q, S>>::set_metrics_sink(self, sink);
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    <QueueMailboxProducer<Q, S>>::set_scheduler_hook(self, hook);
  }
}

impl<M, Q, S> Mailbox<M> for QueueMailbox<Q, S>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, Q, S, M>
  where
    Self: 'a;
  type SendError = QueueError<M>;

  fn try_send(&self, message: M) -> Result<(), Self::SendError> {
    self.inner.try_send(message)
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    QueueMailboxRecv::new(self)
  }

  fn len(&self) -> QueueSize {
    self.inner.len::<M>()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity::<M>()
  }

  fn close(&self) {
    self.inner.close::<M>();
  }

  fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.inner.set_scheduler_hook(hook);
  }
}
