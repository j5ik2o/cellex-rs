use cellex_actor_core_rs::{
  api::{
    mailbox::{
      queue_mailbox::{QueueMailbox, QueueMailboxRecv},
      Mailbox, MailboxError,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::{messages::PriorityEnvelope, MailboxOptions},
};
use cellex_utils_core_rs::collections::{
  queue::{backend::QueueError, QueueSize},
  Element,
};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{
  factory::DefaultPriorityMailboxFactory, priority_mailbox_queue::PriorityMailboxQueue,
  sender::DefaultPriorityMailboxSender,
};
use crate::default_mailbox::DefaultSignal;

/// Mailbox that stores priority envelopes using `ArcShared` storage.
#[derive(Clone)]
pub struct DefaultPriorityMailbox<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  pub(crate) inner: QueueMailbox<(), PriorityMailboxQueue<M>, DefaultSignal<RM>>,
}

impl<M, RM> DefaultPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Creates a mailbox runtime and builds a mailbox with the requested control capacity.
  pub fn new(control_capacity_per_level: usize) -> (Self, DefaultPriorityMailboxSender<M, RM>) {
    DefaultPriorityMailboxFactory::<RM>::new(control_capacity_per_level).mailbox::<M>(MailboxOptions::default())
  }

  /// Returns the underlying queue mailbox.
  pub fn inner(&self) -> &QueueMailbox<(), PriorityMailboxQueue<M>, DefaultSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink::<PriorityEnvelope<M>>(sink);
  }
}

impl<M, RM> Mailbox<PriorityEnvelope<M>> for DefaultPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, (), PriorityMailboxQueue<M>, DefaultSignal<RM>, PriorityEnvelope<M>>
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
    self.inner.set_metrics_sink::<PriorityEnvelope<M>>(sink);
  }
}

impl<M, RM> DefaultPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Sends an envelope using the MailboxError-based API.
  pub fn try_send_mailbox(&self, envelope: PriorityEnvelope<M>) -> Result<(), MailboxError<PriorityEnvelope<M>>> {
    self.inner.try_send_mailbox(envelope)
  }

  /// Returns the receive future when operating with MailboxError semantics.
  pub fn recv_mailbox(
    &self,
  ) -> QueueMailboxRecv<'_, (), PriorityMailboxQueue<M>, DefaultSignal<RM>, PriorityEnvelope<M>> {
    self.inner.recv()
  }
}
