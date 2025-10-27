use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{QueueMailbox, QueueMailboxRecv, SystemMailboxQueue, UserMailboxQueue},
    Mailbox, MailboxError,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_core_rs::collections::{
  queue::{backend::QueueError, QueueSize},
  Element,
};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{factory::DefaultMailboxFactory, sender::DefaultMailboxSender, signal::DefaultSignal};

/// Mailbox implementation backed by an `ArcShared` MPSC queue.
#[derive(Clone)]
pub struct DefaultMailbox<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  pub(crate) inner: QueueMailbox<SystemMailboxQueue<M>, UserMailboxQueue<M>, DefaultSignal<RM>>,
}

impl<M, RM> DefaultMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Creates an unbounded mailbox and sender pair.
  pub fn new() -> (Self, DefaultMailboxSender<M, RM>) {
    DefaultMailboxFactory::<RM>::new().unbounded()
  }

  /// Returns the underlying queue mailbox.
  pub fn inner(&self) -> &QueueMailbox<SystemMailboxQueue<M>, UserMailboxQueue<M>, DefaultSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink::<M>(sink);
  }
}

impl<M, RM> Mailbox<M> for DefaultMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, SystemMailboxQueue<M>, UserMailboxQueue<M>, DefaultSignal<RM>, M>
  where
    Self: 'a;
  type SendError = QueueError<M>;

  fn try_send(&self, message: M) -> Result<(), Self::SendError> {
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
    self.inner.set_metrics_sink::<M>(sink);
  }
}

impl<M, RM> DefaultMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Sends a message using the MailboxError-based API.
  pub fn try_send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    self.inner.try_send_mailbox(message)
  }

  /// Returns the receive future when operating with MailboxError semantics.
  pub fn recv_mailbox(&self) -> QueueMailboxRecv<'_, SystemMailboxQueue<M>, UserMailboxQueue<M>, DefaultSignal<RM>, M> {
    self.inner.recv()
  }
}
