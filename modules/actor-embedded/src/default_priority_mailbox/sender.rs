use cellex_actor_core_rs::{
  api::{
    mailbox::{MailboxError, QueueMailboxProducer},
    metrics::MetricsSinkShared,
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_core_rs::collections::{queue::backend::QueueError, Element};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::priority_mailbox_queue::PriorityMailboxQueue;
use crate::default_mailbox::DefaultSignal;

/// Sending handle associated with [`super::mailbox::DefaultPriorityMailbox`].
#[derive(Clone)]
pub struct DefaultPriorityMailboxSender<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  pub(crate) inner: QueueMailboxProducer<(), PriorityMailboxQueue<M>, DefaultSignal<RM>>,
}

impl<M, RM> DefaultPriorityMailboxSender<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Attempts to enqueue an envelope without blocking.
  pub fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.inner.try_send(message)
  }

  /// Sends an envelope, waiting when required by the backend.
  pub fn send(&self, message: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.inner.send(message)
  }

  /// Attempts to enqueue a user message with the specified priority.
  pub fn try_send_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.try_send(PriorityEnvelope::new(message, priority))
  }

  /// Sends a user message with the specified priority.
  pub fn send_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.send(PriorityEnvelope::new(message, priority))
  }

  /// Attempts to enqueue a control message with the specified priority.
  pub fn try_send_control_with_priority(
    &self,
    message: M,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.try_send(PriorityEnvelope::control(message, priority))
  }

  /// Sends a control message with the specified priority.
  pub fn send_control_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.send(PriorityEnvelope::control(message, priority))
  }

  /// Returns the underlying queue mailbox producer.
  pub fn inner(&self) -> &QueueMailboxProducer<(), PriorityMailboxQueue<M>, DefaultSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink::<PriorityEnvelope<M>>(sink);
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
