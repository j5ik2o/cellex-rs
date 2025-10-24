use core::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use cellex_utils_core_rs::{Element, QueueError, QueueRw};

use super::base::QueueMailbox;
use crate::api::mailbox::MailboxSignal;

/// Future for receiving messages.
pub struct QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element, {
  pub(super) mailbox: &'a QueueMailbox<Q, S>,
  pub(super) wait:    Option<S::WaitFuture<'a>>,
  pub(super) marker:  PhantomData<M>,
}

impl<'a, Q, S, M> QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element,
{
  pub(super) const fn new(mailbox: &'a QueueMailbox<Q, S>) -> Self {
    Self { mailbox, wait: None, marker: PhantomData }
  }
}

impl<'a, Q, S, M> Future for QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element,
{
  type Output = Result<M, QueueError<M>>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = unsafe { self.get_unchecked_mut() };
    if this.mailbox.closed.get() {
      return Poll::Ready(Err(QueueError::Disconnected));
    }
    loop {
      match this.mailbox.queue.poll() {
        | Ok(Some(message)) => {
          this.wait = None;
          return Poll::Ready(Ok(message));
        },
        | Ok(None) => {
          if this.wait.is_none() {
            this.wait = Some(this.mailbox.signal.wait());
          }
        },
        | Err(QueueError::Disconnected) => {
          this.mailbox.closed.set(true);
          this.wait = None;
          return Poll::Ready(Err(QueueError::Disconnected));
        },
        | Err(QueueError::Closed(message)) => {
          this.mailbox.closed.set(true);
          this.wait = None;
          return Poll::Ready(Ok(message));
        },
        | Err(QueueError::Full(_)) | Err(QueueError::OfferError(_)) => return Poll::Pending,
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
