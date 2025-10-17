use crate::api::mailbox::mailbox_signal::MailboxSignal;
use crate::api::mailbox::Mailbox;
use cellex_utils_core_rs::{Element, QueueError};

/// Shared interface exposed by mailbox handles that can be managed by the runtime scheduler.
pub trait MailboxHandle<M>: Mailbox<M> + Clone
where
  M: Element, {
  /// Associated signal type used to block until new messages arrive.
  type Signal: MailboxSignal;

  /// Clones the underlying signal for waiters.
  fn signal(&self) -> Self::Signal;

  /// Attempts to dequeue one message without waiting.
  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>>;
}
