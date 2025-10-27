use cellex_utils_core_rs::collections::{queue::backend::QueueError, Element};

use crate::{
  api::mailbox::{Mailbox, MailboxError},
  shared::mailbox::signal::MailboxSignal,
};

/// Shared interface exposed by mailbox handles that can be managed by the runtime scheduler.
pub trait MailboxHandle<M>: Mailbox<M> + Clone
where
  M: Element, {
  /// Associated signal type used to block until new messages arrive.
  type Signal: MailboxSignal;

  /// Clones the underlying signal for waiters.
  fn signal(&self) -> Self::Signal;

  /// Attempts to dequeue one message without waiting.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the mailbox cannot provide a message due to disconnection.
  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>>;

  /// Attempts to dequeue one message returning the mailbox error model.
  fn try_dequeue_mailbox(&self) -> Result<Option<M>, MailboxError<M>> {
    self.try_dequeue().map_err(MailboxError::from_queue_error)
  }
}
