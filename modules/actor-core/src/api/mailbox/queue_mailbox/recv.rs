use core::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use cellex_utils_core_rs::{collections::queue::QueueError, Element, QueueRw};

use super::{base::QueueMailbox, internal::QueueMailboxInternal, poll_outcome::QueuePollOutcome};
use crate::api::mailbox::MailboxSignal;

/// Future that receives messages from the queue mailbox.
pub struct QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element, {
  pub(super) inner:  &'a QueueMailboxInternal<Q, S>,
  pub(super) wait:   Option<S::WaitFuture<'a>>,
  pub(super) marker: PhantomData<M>,
}

impl<'a, Q, S, M> QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element,
{
  pub(super) const fn new(mailbox: &'a QueueMailbox<Q, S>) -> Self {
    Self { inner: mailbox.inner(), wait: None, marker: PhantomData }
  }

  fn poll_queue(&self) -> QueuePollOutcome<M> {
    QueuePollOutcome::from_result(self.inner.try_dequeue())
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
    if this.inner.is_closed() {
      return Poll::Ready(Err(QueueError::Disconnected));
    }

    loop {
      match this.poll_queue() {
        | QueuePollOutcome::Message(message) => {
          this.wait = None;
          return Poll::Ready(Ok(message));
        },
        | QueuePollOutcome::Empty => {
          if this.wait.is_none() {
            let wait_future = {
              let signal = this.inner.signal();
              signal.wait()
            };
            this.wait = Some(wait_future);
          }
        },
        | QueuePollOutcome::Pending => return Poll::Pending,
        | QueuePollOutcome::Disconnected => {
          this.inner.closed().set(true);
          this.wait = None;
          return Poll::Ready(Err(QueueError::Disconnected));
        },
        | QueuePollOutcome::Closed(message) => {
          this.inner.closed().set(true);
          this.wait = None;
          return Poll::Ready(Ok(message));
        },
        | QueuePollOutcome::Err(QueueError::Full(_)) | QueuePollOutcome::Err(QueueError::OfferError(_)) => {
          return Poll::Pending
        },
        | QueuePollOutcome::Err(QueueError::AllocError(_)) => {
          this.inner.closed().set(true);
          this.wait = None;
          return Poll::Ready(Err(QueueError::Disconnected));
        },
        | QueuePollOutcome::Err(other) => {
          this.inner.closed().set(true);
          this.wait = None;
          return Poll::Ready(Err(other));
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
