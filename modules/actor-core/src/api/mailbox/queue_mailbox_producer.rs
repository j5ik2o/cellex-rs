use cellex_utils_core_rs::{
  collections::Element,
  sync::shared::SharedBound,
  v2::collections::queue::backend::{OfferOutcome, QueueError},
};

use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    mailbox::{
      queue_mailbox::{MailboxQueueCore, MailboxQueueDriver},
      MailboxError,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::MailboxSignal,
};

/// Sending handle that shares queue ownership with
/// [`QueueMailbox`](crate::api::mailbox::queue_mailbox::QueueMailbox).
///
/// Sending handle that shares queue ownership with the mailbox.
/// Allows safe message sending from multiple threads.
///
/// # Type Parameters
/// - `Q`: Message queue implementation type
/// - `S`: Notification signal implementation type
#[derive(Clone)]
pub struct QueueMailboxProducer<Q, S> {
  pub(crate) core: MailboxQueueCore<Q, S>,
}

impl<Q, S> core::fmt::Debug for QueueMailboxProducer<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailboxProducer").finish()
  }
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<Q, S> Send for QueueMailboxProducer<Q, S>
where
  Q: SharedBound,
  S: SharedBound,
{
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<Q, S> Sync for QueueMailboxProducer<Q, S>
where
  Q: SharedBound,
  S: SharedBound,
{
}

impl<Q, S> QueueMailboxProducer<Q, S> {
  pub(crate) fn from_core(core: MailboxQueueCore<Q, S>) -> Self {
    Self { core }
  }

  /// Attempts to send a message and returns the underlying queue outcome.
  pub fn try_send_with_outcome<M>(&self, message: M) -> Result<OfferOutcome, MailboxError<M>>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    self.core.try_send_mailbox(message)
  }

  /// Attempts to send a message (non-blocking) using the mailbox error model.
  pub fn try_send_mailbox<M>(&self, message: M) -> Result<(), MailboxError<M>>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send_with_outcome(message).map(|_| ())
  }

  /// Attempts to send a message (non-blocking).
  ///
  /// Returns an error immediately if the queue is full.
  ///
  /// # Arguments
  /// - `message`: Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, `Err(QueueError)` on failure
  ///
  /// # Errors
  /// - `QueueError::Disconnected`: Mailbox is closed
  /// - `QueueError::Full`: Queue is full
  pub fn try_send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    match self.try_send_with_outcome(message) {
      | Ok(_) => Ok(()),
      | Err(error) => Err(error.into()),
    }
  }

  /// Sends a message using the mailbox queue.
  ///
  /// # Arguments
  /// - `message`: Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, `Err(QueueError)` on failure
  ///
  /// # Errors
  /// Returns [`QueueError`] when the queue rejects the message.
  pub fn send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send(message)
  }

  /// Sends a message using the mailbox error model.
  pub fn send_mailbox<M>(&self, message: M) -> Result<(), MailboxError<M>>
  where
    Q: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send_mailbox(message)
  }

  /// Returns a reference to the underlying queue.
  #[must_use]
  pub fn queue(&self) -> &Q {
    self.core.queue()
  }

  /// Assigns a metrics sink for enqueue instrumentation.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.core.set_metrics_sink(sink);
  }

  /// Installs a scheduler hook for notifying ready queue updates.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.core.set_scheduler_hook(hook);
  }
}
