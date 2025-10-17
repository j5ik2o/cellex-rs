use super::ask::AskResult;
use super::ask_error::AskError;
use super::ask_future::AskFuture;
use cellex_utils_core_rs::Element;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// `AskFuture` wrapper with timeout control.
pub struct AskTimeoutFuture<Resp, TFut> {
  ask: Option<AskFuture<Resp>>,
  timeout: Option<TFut>,
}

impl<Resp, TFut> AskTimeoutFuture<Resp, TFut> {
  pub(super) const fn new(ask: AskFuture<Resp>, timeout: TFut) -> Self {
    Self {
      ask: Some(ask),
      timeout: Some(timeout),
    }
  }
}

impl<Resp, TFut> Future for AskTimeoutFuture<Resp, TFut>
where
  TFut: Future<Output = ()> + Unpin,
  Resp: Element,
{
  type Output = AskResult<Resp>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    if let Some(ask) = self.ask.as_mut() {
      match Pin::new(ask).poll(cx) {
        Poll::Ready(result) => {
          self.ask = None;
          self.timeout = None;
          return Poll::Ready(result);
        }
        Poll::Pending => {}
      }
    }

    if let Some(timeout) = self.timeout.as_mut() {
      if Pin::new(timeout).poll(cx).is_ready() {
        self.timeout = None;
        self.ask.take();
        return Poll::Ready(Err(AskError::Timeout));
      }
    }

    Poll::Pending
  }
}

impl<Resp, TFut> Drop for AskTimeoutFuture<Resp, TFut> {
  fn drop(&mut self) {
    if self.timeout.is_some() {
      self.ask.take();
    }
  }
}
