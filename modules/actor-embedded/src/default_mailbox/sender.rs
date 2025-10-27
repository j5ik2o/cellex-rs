use cellex_actor_core_rs::api::{
  mailbox::{queue_mailbox::SyncMailboxQueue, MailboxError, QueueMailboxProducer},
  metrics::MetricsSinkShared,
};
use cellex_utils_core_rs::collections::{queue::backend::QueueError, Element};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::signal::DefaultSignal;

/// Sending handle associated with [`super::default_mailbox_impl::DefaultMailbox`].
#[derive(Clone)]
pub struct DefaultMailboxSender<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  pub(crate) inner: QueueMailboxProducer<SyncMailboxQueue<M>, DefaultSignal<RM>>,
}

impl<M, RM> DefaultMailboxSender<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Attempts to enqueue a message without blocking.
  pub fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.try_send(message)
  }

  /// Sends a message, waiting for capacity when required.
  pub fn send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.send(message)
  }

  /// Attempts to enqueue a message returning `MailboxError`.
  pub fn try_send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    self.inner.try_send_mailbox(message)
  }

  /// Sends a message returning `MailboxError`.
  pub fn send_mailbox(&self, message: M) -> Result<(), MailboxError<M>> {
    self.inner.send_mailbox(message)
  }

  /// Returns the underlying queue mailbox producer.
  pub fn inner(&self) -> &QueueMailboxProducer<SyncMailboxQueue<M>, DefaultSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}
