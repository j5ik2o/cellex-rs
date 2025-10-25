use cellex_actor_core_rs::{
  api::{
    mailbox::{
      error::MailboxError,
      queue_mailbox::{LegacyQueueDriver, QueueMailbox, QueueMailboxRecv},
      Mailbox, MailboxOptions,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_std_rs::{Element, QueueSize};

use super::{
  factory::TokioPriorityMailboxFactory,
  queues::{self, TokioPriorityQueues},
  sender::TokioPriorityMailboxSender,
  NotifySignal, PriorityQueueError,
};

/// Priority mailbox for Tokio runtime
///
/// An asynchronous mailbox that processes messages based on priority.
/// Control messages are processed with higher priority than regular messages.
pub struct TokioPriorityMailbox<M>
where
  M: Element, {
  inner: QueueMailbox<LegacyQueueDriver<TokioPriorityQueues<M>>, NotifySignal>,
}

impl<M> TokioPriorityMailbox<M>
where
  M: Element,
{
  /// Creates a new priority mailbox
  ///
  /// # Arguments
  ///
  /// * `control_capacity_per_level` - Capacity per priority level for the control queue
  ///
  /// # Returns
  ///
  /// `(TokioPriorityMailbox<M>, TokioPriorityMailboxSender<M>)` - Tuple of mailbox and sender
  /// handle
  #[must_use]
  pub fn new(control_capacity_per_level: usize) -> (Self, TokioPriorityMailboxSender<M>) {
    TokioPriorityMailboxFactory::new(control_capacity_per_level).mailbox::<M>(MailboxOptions::default())
  }

  /// Returns a reference to the internal `QueueMailbox`
  ///
  /// # Returns
  ///
  /// An immutable reference to the internal mailbox
  #[must_use]
  pub const fn inner(&self) -> &QueueMailbox<LegacyQueueDriver<TokioPriorityQueues<M>>, NotifySignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    queues::configure_metrics(self.inner.queue(), sink.clone());
    self.inner.set_metrics_sink(sink);
  }

  /// Creates a new instance from inner components (internal constructor)
  pub(super) fn from_inner(inner: QueueMailbox<LegacyQueueDriver<TokioPriorityQueues<M>>, NotifySignal>) -> Self {
    Self { inner }
  }
}

impl<M> Mailbox<PriorityEnvelope<M>> for TokioPriorityMailbox<M>
where
  M: Element,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, LegacyQueueDriver<TokioPriorityQueues<M>>, NotifySignal, PriorityEnvelope<M>>
  where
    Self: 'a;
  type SendError = PriorityQueueError<M>;

  fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), Self::SendError> {
    self.inner.try_send(message).map_err(Box::new)
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    self.inner.recv()
  }

  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }

  fn close(&self) {
    self.inner.close();
  }

  fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    queues::configure_metrics(self.inner.queue(), sink.clone());
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> TokioPriorityMailbox<M>
where
  M: Element,
{
  /// MailboxError ベースの送信を提供する補助メソッド。
  pub fn try_send_mailbox(
    &self,
    envelope: PriorityEnvelope<M>,
  ) -> Result<(), MailboxError<PriorityEnvelope<M>>> {
    self.inner.try_send_mailbox(envelope)
  }

  /// MailboxError 版の受信 Future を返す。
  pub fn recv_mailbox(&self) -> QueueMailboxRecv<'_, LegacyQueueDriver<TokioPriorityQueues<M>>, NotifySignal, PriorityEnvelope<M>> {
    self.inner.recv()
  }
}
