use cellex_actor_core_rs::{
  api::{
    mailbox::{MailboxError, QueueMailboxProducer},
    metrics::MetricsSinkShared,
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_std_rs::Element;

use super::{
  priority_sync_driver::{configure_metrics, PrioritySyncQueueDriver},
  NotifySignal, PriorityQueueError,
};

type QueueHandle<M> = PrioritySyncQueueDriver<M>;

/// Message sender handle for priority mailbox
///
/// Provides an asynchronous interface for sending messages to the mailbox.
/// Supports sending messages with specified priority and control messages.
pub struct TokioPriorityMailboxSender<M>
where
  M: Element, {
  inner: QueueMailboxProducer<QueueHandle<M>, NotifySignal>,
}

impl<M> TokioPriorityMailboxSender<M>
where
  M: Element,
{
  /// Sends a message in a non-blocking manner
  ///
  /// # Arguments
  ///
  /// * `message` - The priority envelope to send
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully queued, `Err` if the queue is full
  ///
  /// # Errors
  ///
  /// Returns an error if the queue is full or sending fails
  pub fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), PriorityQueueError<M>> {
    self.inner.try_send(message).map_err(Box::new)
  }

  /// Sends a message to the priority mailbox.
  ///
  /// # Arguments
  ///
  /// * `message` - The priority envelope to send
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully sent, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if sending fails
  pub fn send(&self, message: PriorityEnvelope<M>) -> Result<(), PriorityQueueError<M>> {
    self.inner.send(message).map_err(Box::new)
  }

  /// Sends a message with specified priority in a non-blocking manner
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully queued, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if the queue is full or sending fails
  pub fn try_send_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.try_send(PriorityEnvelope::new(message, priority))
  }

  /// Sends a message with specified priority asynchronously
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully sent, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if sending fails
  pub fn send_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.send(PriorityEnvelope::new(message, priority))
  }

  /// Sends a control message with priority in a non-blocking manner
  ///
  /// Control messages are processed with higher priority than regular messages.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully queued, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if the queue is full or sending fails
  pub fn try_send_control_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.try_send(PriorityEnvelope::control(message, priority))
  }

  /// Sends a control message with priority.
  ///
  /// Control messages are processed with higher priority than regular messages.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully sent, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if sending fails
  pub fn send_control_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.send(PriorityEnvelope::control(message, priority))
  }

  /// Returns a reference to the internal `QueueMailboxProducer`
  ///
  /// # Returns
  ///
  /// An immutable reference to the internal producer
  #[must_use]
  pub const fn inner(&self) -> &QueueMailboxProducer<QueueHandle<M>, NotifySignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    configure_metrics(self.inner.queue(), sink.clone());
    self.inner.set_metrics_sink(sink);
  }

  /// Creates a new instance from inner components (internal constructor)
  pub(super) fn from_inner(inner: QueueMailboxProducer<QueueHandle<M>, NotifySignal>) -> Self {
    Self { inner }
  }

  /// Attempts to enqueue using the MailboxError-based API.
  pub fn try_send_mailbox(&self, envelope: PriorityEnvelope<M>) -> Result<(), MailboxError<PriorityEnvelope<M>>> {
    self.inner.try_send_mailbox(envelope)
  }

  /// Sends an envelope using the MailboxError-based API.
  pub fn send_mailbox(&self, envelope: PriorityEnvelope<M>) -> Result<(), MailboxError<PriorityEnvelope<M>>> {
    self.inner.send_mailbox(envelope)
  }
}
