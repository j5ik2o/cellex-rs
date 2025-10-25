use core::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use cellex_utils_core_rs::{collections::queue::QueueError, Element};

use super::{base::QueueMailbox, driver::MailboxQueueDriver};
use crate::api::mailbox::{error::MailboxError, MailboxSignal};

/// Future for receiving messages.
pub struct QueueMailboxRecv<'a, Q, S, M>
where
  Q: MailboxQueueDriver<M>,
  S: MailboxSignal,
  M: Element, {
  pub(super) mailbox: &'a QueueMailbox<Q, S>,
  pub(super) wait:    Option<S::WaitFuture<'a>>,
  pub(super) marker:  PhantomData<M>,
}

impl<'a, Q, S, M> QueueMailboxRecv<'a, Q, S, M>
where
  Q: MailboxQueueDriver<M>,
  S: MailboxSignal,
  M: Element,
{
  pub(super) const fn new(mailbox: &'a QueueMailbox<Q, S>) -> Self {
    Self { mailbox, wait: None, marker: PhantomData }
  }
}

impl<'a, Q, S, M> Future for QueueMailboxRecv<'a, Q, S, M>
where
  Q: MailboxQueueDriver<M>,
  S: MailboxSignal,
  M: Element,
{
  type Output = Result<M, QueueError<M>>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = unsafe { self.get_unchecked_mut() };
    if this.mailbox.core.closed().get() {
      return Poll::Ready(Err(QueueError::Disconnected));
    }
    loop {
      match this.mailbox.core.try_dequeue_mailbox() {
        | Ok(Some(message)) => {
          this.wait = None;
          return Poll::Ready(Ok(message));
        },
        | Ok(None) => {
          if this.wait.is_none() {
            this.wait = Some(this.mailbox.core.signal().wait());
          }
        },
        | Err(mailbox_error) => match Self::map_mailbox_error(mailbox_error) {
          | QueueMailboxRecvOutcome::Message(message) => {
            this.wait = None;
            return Poll::Ready(Ok(message));
          },
          | QueueMailboxRecvOutcome::Retry => return Poll::Pending,
          | QueueMailboxRecvOutcome::Disconnected => {
            this.mailbox.core.closed().set(true);
            this.wait = None;
            return Poll::Ready(Err(QueueError::Disconnected));
          },
        },
      }

      if let Some(wait) = this.wait.as_mut() {
        match unsafe { Pin::new_unchecked(wait) }.poll(cx) {
          | Poll::Ready(()) => {
            this.wait = None;
            continue;
          },
          | Poll::Pending => return Poll::Pending,
        }
      }
    }
  }
}

enum QueueMailboxRecvOutcome<M> {
  Message(M),
  Retry,
  Disconnected,
}

impl<'a, Q, S, M> QueueMailboxRecv<'a, Q, S, M>
where
  Q: MailboxQueueDriver<M>,
  S: MailboxSignal,
  M: Element,
{
  fn map_mailbox_error(error: MailboxError<M>) -> QueueMailboxRecvOutcome<M> {
    match error {
      | MailboxError::QueueFull { .. } => QueueMailboxRecvOutcome::Retry,
      | MailboxError::Disconnected => QueueMailboxRecvOutcome::Disconnected,
      | MailboxError::Closed { last: Some(message) } => QueueMailboxRecvOutcome::Message(message),
      | MailboxError::Closed { last: None } => QueueMailboxRecvOutcome::Disconnected,
      | MailboxError::Backpressure => QueueMailboxRecvOutcome::Retry,
      | MailboxError::ResourceExhausted { .. } => QueueMailboxRecvOutcome::Disconnected,
      | MailboxError::Internal { .. } => QueueMailboxRecvOutcome::Retry,
    }
  }
}
