use cellex_utils_core_rs::{
  collections::{queue::QueueSize, Element},
  v2::collections::queue::backend::QueueError,
};

use super::{core::MailboxQueueCore, driver::MailboxQueueDriver, recv::QueueMailboxRecv};
use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    mailbox::{queue_mailbox_producer::QueueMailboxProducer, Mailbox, MailboxError},
    metrics::MetricsSinkShared,
  },
  shared::mailbox::{MailboxHandle, MailboxProducer, MailboxSignal},
};

/// Mailbox backed by a queue and notification signal.
pub struct QueueMailbox<Q, S> {
  pub(super) core: MailboxQueueCore<Q, S>,
}

impl<Q, S> QueueMailbox<Q, S> {
  /// Creates a new queue mailbox.
  pub fn new(queue: Q, signal: S) -> Self {
    Self { core: MailboxQueueCore::new(queue, signal) }
  }

  /// Gets a reference to the internal queue.
  #[must_use]
  pub const fn queue(&self) -> &Q {
    self.core.queue()
  }

  /// Gets a reference to the internal signal.
  #[must_use]
  pub const fn signal(&self) -> &S {
    self.core.signal()
  }

  /// Creates a producer handle for sending messages.
  pub fn producer(&self) -> QueueMailboxProducer<Q, S>
  where
    Q: Clone,
    S: Clone, {
    QueueMailboxProducer::from_core(self.core.clone())
  }

  /// Attempts to enqueue a message returning `MailboxError`.
  pub fn try_send_mailbox<M>(&self, message: M) -> Result<(), MailboxError<M>>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    self.core.try_send_mailbox(message).map(|_| ())
  }

  /// Configures a metrics sink used for enqueue instrumentation.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.core.set_metrics_sink(sink);
  }

  /// Installs a scheduler hook that is notified when new messages arrive.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.core.set_scheduler_hook(hook);
  }

  /// Returns the current queue length as `usize`.
  #[must_use]
  pub fn len_usize<M>(&self) -> usize
  where
    Q: MailboxQueueDriver<M>,
    M: Element, {
    self.core.len::<M>().to_usize()
  }

  /// Returns the queue capacity as `usize`.
  #[must_use]
  pub fn capacity_usize<M>(&self) -> usize
  where
    Q: MailboxQueueDriver<M>,
    M: Element, {
    self.core.capacity::<M>().to_usize()
  }

  /// Returns the mailbox receive future that yields `MailboxError` on failure.
  pub fn recv_mailbox<'a, M>(&'a self) -> QueueMailboxRecv<'a, Q, S, M>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    QueueMailboxRecv::new(self)
  }
}

impl<Q, S> Clone for QueueMailbox<Q, S>
where
  Q: Clone,
  S: Clone,
{
  fn clone(&self) -> Self {
    Self { core: self.core.clone() }
  }
}

impl<Q, S> core::fmt::Debug for QueueMailbox<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailbox").finish()
  }
}

impl<M, Q, S> MailboxHandle<M> for QueueMailbox<Q, S>
where
  Q: MailboxQueueDriver<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  type Signal = S;

  fn signal(&self) -> Self::Signal {
    self.signal().clone()
  }

  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>> {
    self.core.try_dequeue()
  }
}

impl<M, Q, S> MailboxProducer<M> for QueueMailboxProducer<Q, S>
where
  Q: MailboxQueueDriver<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    <QueueMailboxProducer<Q, S>>::try_send(self, message)
  }

  fn try_send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    <QueueMailboxProducer<Q, S>>::try_send_mailbox(self, message)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    let queue_sink = sink.clone();
    self.core.queue().set_metrics_sink(queue_sink);
    self.core.set_metrics_sink(sink);
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    <QueueMailboxProducer<Q, S>>::set_scheduler_hook(self, hook);
  }
}

impl<M, Q, S> Mailbox<M> for QueueMailbox<Q, S>
where
  Q: MailboxQueueDriver<M>,
  S: MailboxSignal,
  M: Element,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, Q, S, M>
  where
    Self: 'a;
  type SendError = QueueError<M>;

  fn try_send(&self, message: M) -> Result<(), Self::SendError> {
    self.core.try_send(message)
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    QueueMailboxRecv::new(self)
  }

  fn len(&self) -> QueueSize {
    self.core.len()
  }

  fn capacity(&self) -> QueueSize {
    self.core.capacity()
  }

  fn close(&self) {
    self.core.close()
  }

  fn is_closed(&self) -> bool {
    self.core.is_closed()
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    let queue_sink = sink.clone();
    self.core.queue().set_metrics_sink(queue_sink);
    self.core.set_metrics_sink(sink);
  }
}
