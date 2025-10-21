use cellex_utils_core_rs::{Element, QueueError};

use crate::api::{actor_scheduler::ready_queue_scheduler::ReadyQueueHandle, metrics::MetricsSinkShared};

/// Sending interface exposed by mailbox producers that enqueue messages.
pub trait MailboxProducer<M>: Clone
where
  M: Element, {
  /// Attempts to enqueue a message without waiting.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the mailbox cannot accept the message.
  fn try_send(&self, message: M) -> Result<(), QueueError<M>>;

  /// Injects a metrics sink for enqueue instrumentation. Default: no-op.
  fn set_metrics_sink(&mut self, _sink: Option<MetricsSinkShared>) {}

  /// Installs a scheduler hook invoked on message arrivals. Default: no-op.
  fn set_scheduler_hook(&mut self, _hook: Option<ReadyQueueHandle>) {}
}
