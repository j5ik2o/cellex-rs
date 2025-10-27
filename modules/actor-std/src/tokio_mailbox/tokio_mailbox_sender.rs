use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{MailboxQueueBackend, SyncMailboxQueue},
    MailboxError, QueueMailboxProducer,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_core_rs::collections::{queue::backend::QueueError, Element};

use super::notify_signal::NotifySignal;

type TokioQueueDriver<M> = SyncMailboxQueue<M>;

/// Sender handle for Tokio mailbox
///
/// Provides an interface specialized for sending messages.
#[derive(Clone, Debug)]
pub struct TokioMailboxSender<M>
where
  M: Element, {
  pub(super) inner: QueueMailboxProducer<TokioQueueDriver<M>, NotifySignal>,
}

impl<M> TokioMailboxSender<M>
where
  M: Element,
  TokioQueueDriver<M>: Clone,
{
  /// Attempts to send a message (non-blocking)
  ///
  /// # Arguments
  /// * `message` - The message to send
  ///
  /// # Returns
  /// `Ok(())` on success, or an error with the message on failure
  ///
  /// # Errors
  /// Returns `QueueError::Full` if the queue is full
  pub fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.try_send(message)
  }

  /// Sends a message to the mailbox.
  ///
  /// # Arguments
  /// * `message` - The message to send
  ///
  /// # Returns
  /// `Ok(())` on success, or an error with the message on failure
  ///
  /// # Errors
  /// Returns `QueueError::Closed` if the mailbox is closed
  pub fn send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.send(message)
  }

  /// Attempts to send a message returning `MailboxError` for new API consumers.
  pub fn try_send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    self.inner.try_send_mailbox(message)
  }

  /// Sends a message returning `MailboxError` for new API consumers.
  pub fn send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    self.inner.send_mailbox(message)
  }

  /// Returns a reference to the internal queue mailbox producer
  ///
  /// # Returns
  /// An immutable reference to the internal producer
  #[must_use]
  pub const fn inner(&self) -> &QueueMailboxProducer<TokioQueueDriver<M>, NotifySignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.queue().set_metrics_sink(sink.clone());
    self.inner.set_metrics_sink(sink);
  }
}
