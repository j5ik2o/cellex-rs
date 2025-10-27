use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{MailboxQueueBackend, QueueMailboxRecv, SyncMailbox, SyncMailboxQueue},
    Mailbox, MailboxError,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_core_rs::collections::{
  queue::{backend::QueueError, QueueSize},
  Element,
};

use super::{
  notify_signal::NotifySignal, tokio_mailbox_factory::TokioMailboxFactory, tokio_mailbox_sender::TokioMailboxSender,
};

type TokioQueueDriver<M> = SyncMailboxQueue<M>;
type TokioMailboxInner<M> = SyncMailbox<M, NotifySignal>;

/// Mailbox implementation for Tokio runtime
///
/// An asynchronous queue that manages message delivery to actors.
#[derive(Clone, Debug)]
pub struct TokioMailbox<M>
where
  M: Element, {
  pub(super) inner: TokioMailboxInner<M>,
}

impl<M> TokioMailbox<M>
where
  M: Element,
{
  /// Creates a mailbox with the specified capacity
  ///
  /// # Arguments
  /// * `capacity` - Maximum capacity of the mailbox
  ///
  /// # Returns
  /// A pair of mailbox and sender handle
  #[must_use]
  pub fn new(capacity: usize) -> (Self, TokioMailboxSender<M>) {
    TokioMailboxFactory.with_capacity(capacity)
  }

  /// Creates an unbounded mailbox
  ///
  /// # Returns
  /// A pair of mailbox and sender handle
  #[must_use]
  pub fn unbounded() -> (Self, TokioMailboxSender<M>) {
    TokioMailboxFactory.unbounded()
  }

  /// Creates a new sender handle
  ///
  /// # Returns
  /// A `TokioMailboxSender` for sending messages
  #[must_use]
  pub fn producer(&self) -> TokioMailboxSender<M>
  where
    TokioQueueDriver<M>: Clone,
    NotifySignal: Clone, {
    TokioMailboxSender { inner: self.inner.producer() }
  }

  /// Returns a reference to the internal queue mailbox
  ///
  /// # Returns
  /// An immutable reference to the internal mailbox
  #[must_use]
  pub const fn inner(&self) -> &TokioMailboxInner<M> {
    &self.inner
  }
}

impl<M> Mailbox<M> for TokioMailbox<M>
where
  M: Element,
  TokioQueueDriver<M>: Clone,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, TokioQueueDriver<M>, NotifySignal, M>
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
    self.inner.queue().set_metrics_sink(sink.clone());
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> TokioMailbox<M>
where
  M: Element,
  TokioQueueDriver<M>: Clone,
{
  /// Sends a message using the MailboxError-based API.
  pub fn try_send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    self.inner.try_send_mailbox(message)
  }

  /// Returns the receive future when working with MailboxError-based semantics.
  pub fn recv_mailbox(&self) -> QueueMailboxRecv<'_, TokioQueueDriver<M>, NotifySignal, M> {
    self.inner.recv()
  }
}
