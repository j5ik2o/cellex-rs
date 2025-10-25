use cellex_utils_core_rs::{collections::queue::QueueError, Element};

use crate::{api::mailbox::Mailbox, shared::mailbox::signal::MailboxSignal};

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
}
