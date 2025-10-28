use cellex_utils_core_rs::{
  collections::{
    queue::backend::{OfferOutcome, QueueError},
    Element,
  },
  sync::shared::SharedBound,
};

use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    mailbox::{
      queue_mailbox::{MailboxQueue, QueueMailboxCore, SystemMailboxLane},
      MailboxError,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::MailboxSignal,
};

/// Sending handle that shares queue ownership with
/// [`QueueMailbox`](crate::api::mailbox::queue_mailbox::QueueMailbox).
#[derive(Clone)]
pub struct QueueMailboxProducer<SQ, UQ, S> {
  pub(crate) core: QueueMailboxCore<SQ, UQ, S>,
}

impl<SQ, UQ, S> core::fmt::Debug for QueueMailboxProducer<SQ, UQ, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailboxProducer").finish()
  }
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<SQ, UQ, S> Send for QueueMailboxProducer<SQ, UQ, S>
where
  SQ: SharedBound,
  UQ: SharedBound,
  S: SharedBound,
{
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<SQ, UQ, S> Sync for QueueMailboxProducer<SQ, UQ, S>
where
  SQ: SharedBound,
  UQ: SharedBound,
  S: SharedBound,
{
}

impl<SQ, UQ, S> QueueMailboxProducer<SQ, UQ, S> {
  pub(crate) fn from_core(core: QueueMailboxCore<SQ, UQ, S>) -> Self {
    Self { core }
  }

  /// Returns a reference to the system queue when available.
  #[must_use]
  pub const fn system_queue(&self) -> Option<&SQ> {
    self.core.system_queue()
  }

  /// Returns a reference to the user queue.
  #[must_use]
  pub const fn user_queue(&self) -> &UQ {
    self.core.user_queue()
  }

  /// Assigns a metrics sink for enqueue instrumentation.
  pub fn set_metrics_sink<M>(&mut self, sink: Option<MetricsSinkShared>)
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    self.core.apply_queue_metrics_sink::<M>(sink.clone());
    self.core.set_metrics_sink(sink);
  }

  /// Installs a scheduler hook for notifying ready queue updates.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.core.set_scheduler_hook(hook);
  }
}

impl<SQ, UQ, S> QueueMailboxProducer<SQ, UQ, S> {
  /// Attempts to send a message and returns the underlying queue outcome.
  pub fn try_send_with_outcome<M>(&self, message: M) -> Result<OfferOutcome, MailboxError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    self.core.try_send_mailbox(message)
  }

  /// Attempts to send a message (non-blocking) using the mailbox error model.
  pub fn try_send_mailbox<M>(&self, message: M) -> Result<(), MailboxError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send_with_outcome(message).map(|_| ())
  }

  /// Attempts to send a message (non-blocking).
  ///
  /// Returns an error immediately if the queue is full.
  pub fn try_send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    match self.try_send_with_outcome(message) {
      | Ok(_) => Ok(()),
      | Err(error) => Err(error.into()),
    }
  }

  /// Sends a message using the mailbox queue.
  pub fn send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send(message)
  }

  /// Sends a message using the mailbox error model.
  pub fn send_mailbox<M>(&self, message: M) -> Result<(), MailboxError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send_mailbox(message)
  }
}
