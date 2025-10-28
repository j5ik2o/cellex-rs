use cellex_utils_core_rs::collections::{
  queue::{backend::QueueError, QueueSize},
  Element,
};

use super::{core::QueueMailboxCore, queue::MailboxQueue, recv::QueueMailboxRecv, SystemMailboxLane};
use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    mailbox::{queue_mailbox_producer::QueueMailboxProducer, Mailbox, MailboxError},
    metrics::MetricsSinkShared,
  },
  shared::mailbox::{MailboxConsumer, MailboxProducer, MailboxSignal},
};

/// Mailbox backed by distinct system and user queues plus a notification signal.
pub struct QueueMailbox<SQ, UQ, S> {
  pub(super) core: QueueMailboxCore<SQ, UQ, S>,
}

impl<SQ, UQ, S> QueueMailbox<SQ, UQ, S> {
  /// Creates a mailbox without a system queue.
  pub fn new(user_queue: UQ, signal: S) -> Self {
    Self::with_queues(None, user_queue, signal)
  }

  /// Creates a mailbox with a system queue.
  pub fn with_system_queue(system_queue: SQ, user_queue: UQ, signal: S) -> Self {
    Self::with_queues(Some(system_queue), user_queue, signal)
  }

  /// Creates a mailbox with the supplied queues and signal.
  pub fn with_queues(system_queue: Option<SQ>, user_queue: UQ, signal: S) -> Self {
    Self { core: QueueMailboxCore::new(system_queue, user_queue, signal) }
  }

  /// Gets a reference to the system queue.
  #[must_use]
  pub const fn system_queue(&self) -> Option<&SQ> {
    self.core.system_queue()
  }

  /// Gets a reference to the user queue.
  #[must_use]
  pub const fn user_queue(&self) -> &UQ {
    self.core.user_queue()
  }

  /// Gets a reference to the internal signal.
  #[must_use]
  pub const fn signal(&self) -> &S {
    self.core.signal()
  }

  /// Creates a producer handle for sending messages.
  pub fn producer(&self) -> QueueMailboxProducer<SQ, UQ, S>
  where
    SQ: Clone,
    UQ: Clone,
    S: Clone, {
    QueueMailboxProducer::from_core(self.core.clone())
  }

  /// Configures a metrics sink used for enqueue instrumentation.
  pub fn set_metrics_sink<M>(&mut self, sink: Option<MetricsSinkShared>)
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    self.core.apply_queue_metrics_sink::<M>(sink.clone());
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
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    self.core.len::<M>().to_usize()
  }

  /// Returns the queue capacity as `usize`.
  #[must_use]
  pub fn capacity_usize<M>(&self) -> usize
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    self.core.capacity::<M>().to_usize()
  }

  /// Attempts to enqueue a message returning `MailboxError`.
  pub fn try_send_mailbox<M>(&self, message: M) -> Result<(), MailboxError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    self.core.try_send_mailbox(message).map(|_| ())
  }

  /// Returns the mailbox receive future that yields `MailboxError` on failure.
  pub fn recv_mailbox<'a, M>(&'a self) -> QueueMailboxRecv<'a, SQ, UQ, S, M>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    QueueMailboxRecv::new(self)
  }
}

impl<SQ, UQ, S> Clone for QueueMailbox<SQ, UQ, S>
where
  SQ: Clone,
  UQ: Clone,
  S: Clone,
{
  fn clone(&self) -> Self {
    Self { core: self.core.clone() }
  }
}

impl<SQ, UQ, S> core::fmt::Debug for QueueMailbox<SQ, UQ, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailbox").finish()
  }
}

impl<M, SQ, UQ, S> MailboxConsumer<M> for QueueMailbox<SQ, UQ, S>
where
  SQ: SystemMailboxLane<M> + Clone,
  UQ: MailboxQueue<M> + Clone,
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

impl<M, SQ, UQ, S> MailboxProducer<M> for QueueMailboxProducer<SQ, UQ, S>
where
  SQ: SystemMailboxLane<M> + Clone,
  UQ: MailboxQueue<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    <QueueMailboxProducer<SQ, UQ, S>>::try_send(self, message)
  }

  fn try_send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    <QueueMailboxProducer<SQ, UQ, S>>::try_send_mailbox(self, message)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    let queue_sink = sink.clone();
    self.core.apply_queue_metrics_sink::<M>(queue_sink);
    self.core.set_metrics_sink(sink);
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    <QueueMailboxProducer<SQ, UQ, S>>::set_scheduler_hook(self, hook);
  }
}

impl<M, SQ, UQ, S> Mailbox<M> for QueueMailbox<SQ, UQ, S>
where
  SQ: SystemMailboxLane<M>,
  UQ: MailboxQueue<M>,
  S: MailboxSignal,
  M: Element,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, SQ, UQ, S, M>
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
    self.core.apply_queue_metrics_sink::<M>(queue_sink);
    self.core.set_metrics_sink(sink);
  }
}
