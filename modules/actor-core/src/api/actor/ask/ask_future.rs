use super::ask::AskResult;
use super::ask_error::AskError;
use super::shared::{AskShared, STATE_CANCELLED, STATE_PENDING, STATE_READY, STATE_RESPONDER_DROPPED};
use cellex_utils_core_rs::sync::ArcShared;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// Future that awaits a response from an `ask` operation.
pub struct AskFuture<Resp> {
  pub(super) shared: ArcShared<AskShared<Resp>>,
}

impl<Resp> AskFuture<Resp> {
  pub(super) const fn new(shared: ArcShared<AskShared<Resp>>) -> Self {
    Self { shared }
  }
}

impl<Resp> Future for AskFuture<Resp> {
  type Output = AskResult<Resp>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let shared = &self.shared;

    loop {
      match shared.state() {
        STATE_READY => {
          let value = unsafe { shared.take_value() };
          if let Some(value) = value {
            return Poll::Ready(Ok(value));
          }
          return Poll::Ready(Err(AskError::MissingResponder));
        }
        STATE_RESPONDER_DROPPED => return Poll::Ready(Err(AskError::ResponderDropped)),
        STATE_CANCELLED => return Poll::Ready(Err(AskError::ResponseAwaitCancelled)),
        STATE_PENDING => {
          shared.waker.register(cx.waker());
          if shared.state() == STATE_PENDING {
            return Poll::Pending;
          }
        }
        _ => return Poll::Ready(Err(AskError::ResponseAwaitCancelled)),
      }
    }
  }
}

impl<Resp> Drop for AskFuture<Resp> {
  fn drop(&mut self) {
    let _ = self.shared.cancel();
  }
}

unsafe impl<Resp> Send for AskFuture<Resp> where Resp: Send {}
unsafe impl<Resp> Sync for AskFuture<Resp> where Resp: Send {}
impl<Resp> Unpin for AskFuture<Resp> {}
