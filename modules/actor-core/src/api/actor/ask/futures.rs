use super::error::AskError;
use super::shared::{AskShared, STATE_CANCELLED, STATE_PENDING, STATE_READY, STATE_RESPONDER_DROPPED};
use super::AskResult;
use crate::api::mailbox::MailboxConcurrency;
use crate::api::{InternalMessageSender, MessageEnvelope, MessageSender};
use crate::internal::message::{discard_metadata, DynMessage};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};
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

/// Helper function to create an `AskFuture` with timeout.
pub const fn ask_with_timeout<Resp, TFut>(future: AskFuture<Resp>, timeout: TFut) -> AskTimeoutFuture<Resp, TFut>
where
  TFut: Future<Output = ()> + Unpin,
  Resp: Element, {
  AskTimeoutFuture::new(future, timeout)
}

/// Creates a Future and responder pair for the `ask` pattern (internal API).
pub(crate) fn create_ask_handles<Resp, C>() -> (AskFuture<Resp>, MessageSender<Resp, C>)
where
  Resp: Element,
  C: MailboxConcurrency, {
  let shared = ArcShared::new(AskShared::<Resp>::new());
  let future = AskFuture::new(shared.clone());
  let dispatch_state = shared.clone();
  let drop_state = shared;

  let dispatch = ArcShared::new(move |message: DynMessage, _priority: i8| {
    let Ok(envelope) = message.downcast::<MessageEnvelope<Resp>>() else {
      return Err(QueueError::Disconnected);
    };
    match envelope {
      MessageEnvelope::User(user) => {
        let (value, metadata_key) = user.into_parts();
        if let Some(key) = metadata_key {
          discard_metadata(key);
        }
        if !dispatch_state.complete(value) {
          // response already handled
        }
      }
      MessageEnvelope::System(_) => {
        return Err(QueueError::Disconnected);
      }
    }
    Ok(())
  })
  .into_dyn(|f| f as &super::shared::DispatchFn);

  let drop_hook = ArcShared::new(move || {
    drop_state.responder_dropped();
  })
  .into_dyn(|f| f as &super::shared::DropHookFn);

  let internal = InternalMessageSender::<C>::with_drop_hook(dispatch, drop_hook);
  let responder = MessageSender::new(internal);
  (future, responder)
}
