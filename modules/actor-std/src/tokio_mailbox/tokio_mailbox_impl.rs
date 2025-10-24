use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{QueueMailbox, QueueMailboxRecv},
    Mailbox,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_std_rs::{Element, QueueError, QueueSize};

use super::{
  notify_signal::NotifySignal,
  tokio_mailbox_factory::TokioMailboxFactory,
  tokio_mailbox_sender::TokioMailboxSender,
  tokio_queue::{self, TokioQueue},
};

/// Mailbox implementation for Tokio runtime
///
/// An asynchronous queue that manages message delivery to actors.
#[derive(Clone, Debug)]
pub struct TokioMailbox<M>
where
  M: Element, {
  pub(super) inner: QueueMailbox<TokioQueue<M>, NotifySignal>,
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
    TokioQueue<M>: Clone,
    NotifySignal: Clone, {
    TokioMailboxSender { inner: self.inner.producer() }
  }

  /// Returns a reference to the internal queue mailbox
  ///
  /// # Returns
  /// An immutable reference to the internal mailbox
  #[must_use]
  pub const fn inner(&self) -> &QueueMailbox<TokioQueue<M>, NotifySignal> {
    &self.inner
  }
}

impl<M> Mailbox<M> for TokioMailbox<M>
where
  M: Element,
  TokioQueue<M>: Clone,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, TokioQueue<M>, NotifySignal, M>
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
    tokio_queue::configure_metrics(self.inner.queue(), sink.clone());
    self.inner.set_metrics_sink(sink);
  }
}
