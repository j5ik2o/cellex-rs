use core::fmt;

use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{QueueMailbox, QueueMailboxRecv},
    Mailbox,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_embedded_rs::{Element, QueueError, QueueSize};

use super::{
  local_mailbox_factory::LocalMailboxFactory, local_mailbox_sender::LocalMailboxSender, local_queue::LocalQueue,
  local_signal::LocalSignal,
};

/// Asynchronous mailbox for local thread.
///
/// Uses `Rc`-based queue to deliver messages in `!Send` environments.
pub struct LocalMailbox<M>
where
  M: Element, {
  pub(super) inner: QueueMailbox<LocalQueue<M>, LocalSignal>,
}

impl<M> LocalMailbox<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  /// Creates a new mailbox pair with default settings.
  ///
  /// # Returns
  ///
  /// A tuple of (receiver mailbox, sender handle)
  #[must_use]
  pub fn new() -> (Self, LocalMailboxSender<M>) {
    LocalMailboxFactory::default().unbounded()
  }

  /// Creates a new sender handle.
  ///
  /// # Returns
  ///
  /// A new sender to the mailbox
  #[must_use]
  pub fn producer(&self) -> LocalMailboxSender<M>
  where
    LocalSignal: Clone, {
    LocalMailboxSender { inner: self.inner.producer() }
  }

  /// Returns a reference to the internal queue mailbox.
  ///
  /// # Returns
  ///
  /// A reference to the `QueueMailbox`
  #[must_use]
  pub const fn inner(&self) -> &QueueMailbox<LocalQueue<M>, LocalSignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> Mailbox<M> for LocalMailbox<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, LocalQueue<M>, LocalSignal, M>
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
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> Clone for LocalMailbox<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl<M> fmt::Debug for LocalMailbox<M>
where
  M: Element,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LocalMailbox").finish()
  }
}
