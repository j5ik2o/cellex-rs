use cellex_actor_core_rs::{
  api::{
    mailbox::{
      queue_mailbox::{LegacyQueueDriver, QueueMailbox, QueueMailboxRecv},
      Mailbox, MailboxOptions,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_embedded_rs::{Element, QueueError, QueueSize};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{factory::ArcPriorityMailboxFactory, queues::ArcPriorityQueues, sender::ArcPriorityMailboxSender};
use crate::arc_mailbox::ArcSignal;

/// Mailbox that stores priority envelopes using `ArcShared` storage.
#[derive(Clone)]
pub struct ArcPriorityMailbox<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  pub(crate) inner: QueueMailbox<LegacyQueueDriver<ArcPriorityQueues<M, RM>>, ArcSignal<RM>>,
}

impl<M, RM> ArcPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Creates a mailbox runtime and builds a mailbox with the requested control capacity.
  pub fn new(control_capacity_per_level: usize) -> (Self, ArcPriorityMailboxSender<M, RM>) {
    ArcPriorityMailboxFactory::<RM>::new(control_capacity_per_level).mailbox(MailboxOptions::default())
  }

  /// Returns the underlying queue mailbox.
  pub fn inner(&self) -> &QueueMailbox<LegacyQueueDriver<ArcPriorityQueues<M, RM>>, ArcSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M, RM> Mailbox<PriorityEnvelope<M>> for ArcPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, LegacyQueueDriver<ArcPriorityQueues<M, RM>>, ArcSignal<RM>, PriorityEnvelope<M>>
  where
    Self: 'a;
  type SendError = QueueError<PriorityEnvelope<M>>;

  fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), Self::SendError> {
    self.inner.try_send(message)
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
    self.inner.set_metrics_sink(sink);
  }
}
