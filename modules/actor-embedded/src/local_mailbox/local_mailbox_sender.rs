use core::fmt;

use cellex_actor_core_rs::api::{
  mailbox::{queue_mailbox::LegacyQueueDriver, QueueMailboxProducer},
  metrics::MetricsSinkShared,
};
use cellex_utils_embedded_rs::{Element, QueueError};

use super::{local_queue::LocalQueue, local_signal::LocalSignal};

/// Message sender to `LocalMailbox`.
///
/// A handle for sending messages to the mailbox asynchronously.
pub struct LocalMailboxSender<M>
where
  M: Element, {
  pub(super) inner: QueueMailboxProducer<LegacyQueueDriver<LocalQueue<M>>, LocalSignal>,
}

impl<M> LocalMailboxSender<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  /// Sends a message immediately (non-blocking).
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  ///
  /// # Errors
  ///
  /// Returns `QueueError` if the queue is full or closed
  pub fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.try_send(message)
  }

  /// Sends a message to the mailbox.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  ///
  /// # Errors
  ///
  /// Returns `QueueError` if the queue is closed
  pub fn send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.send(message)
  }

  /// Returns a reference to the internal mailbox producer.
  ///
  /// # Returns
  ///
  /// A reference to the `QueueMailboxProducer`
  #[must_use]
  pub const fn inner(&self) -> &QueueMailboxProducer<LegacyQueueDriver<LocalQueue<M>>, LocalSignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> Clone for LocalMailboxSender<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl<M> fmt::Debug for LocalMailboxSender<M>
where
  M: Element,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LocalMailboxSender").finish()
  }
}
