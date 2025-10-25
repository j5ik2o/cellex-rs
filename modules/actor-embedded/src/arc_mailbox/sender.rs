use cellex_actor_core_rs::api::{mailbox::{queue_mailbox::LegacyQueueDriver, QueueMailboxProducer}, metrics::MetricsSinkShared};
use cellex_utils_embedded_rs::{collections::queue::mpsc::ArcMpscUnboundedQueue, Element, QueueError};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::signal::ArcSignal;

/// Sending handle associated with [`super::arc_mailbox_impl::ArcMailbox`].
#[derive(Clone)]
pub struct ArcMailboxSender<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex,
{
  pub(crate) inner: QueueMailboxProducer<LegacyQueueDriver<ArcMpscUnboundedQueue<M, RM>>, ArcSignal<RM>>,
}

impl<M, RM> ArcMailboxSender<M, RM>
where
  M: Element,
  RM: RawMutex,
  ArcMpscUnboundedQueue<M, RM>: Clone,
{
  /// Attempts to enqueue a message without blocking.
  pub fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.try_send(message)
  }

  /// Sends a message, waiting for capacity when required.
  pub fn send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.send(message)
  }

  /// Returns the underlying queue mailbox producer.
  pub fn inner(&self) -> &QueueMailboxProducer<LegacyQueueDriver<ArcMpscUnboundedQueue<M, RM>>, ArcSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}
