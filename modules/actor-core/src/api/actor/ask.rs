mod ask_error;
mod ask_future;
mod ask_timeout_future;
mod shared;

pub use ask_error::AskError;
pub use ask_future::AskFuture;
pub use ask_timeout_future::AskTimeoutFuture;
pub(crate) use shared::{AskShared, DispatchFn, DropHookFn};

/// Result alias used by `ask` helpers.
pub type AskResult<T> = Result<T, AskError>;

use core::future::Future;

use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};

use crate::{
  api::messaging::{MessageEnvelope, MessageSender, MetadataStorageMode},
  internal::message::InternalMessageSender,
  shared::messaging::AnyMessage,
};

/// Helper function to create an `AskFuture` with timeout.
pub const fn ask_with_timeout<Resp, TFut>(future: AskFuture<Resp>, timeout: TFut) -> AskTimeoutFuture<Resp, TFut>
where
  TFut: Future<Output = ()> + Unpin,
  Resp: Element, {
  AskTimeoutFuture::new(future, timeout)
}

/// Creates a Future and responder pair for the `ask` pattern (internal API).
pub(crate) fn create_ask_handles<Resp, Mode>() -> (AskFuture<Resp>, MessageSender<Resp, Mode>)
where
  Resp: Element,
  Mode: MetadataStorageMode, {
  let shared = ArcShared::new(AskShared::<Resp>::new());
  let future = AskFuture::new(shared.clone());
  let dispatch_state = shared.clone();
  let drop_state = shared;

  let dispatch = ArcShared::new(move |message: AnyMessage, _priority: i8| {
    let Ok(envelope) = message.downcast::<MessageEnvelope<Resp>>() else {
      return Err(QueueError::Disconnected);
    };
    match envelope {
      | MessageEnvelope::User(user) => {
        let (value, _metadata) = user.into_parts::<Mode>();
        if !dispatch_state.complete(value) {
          // response already handled
        }
      },
      | MessageEnvelope::System(_) => {
        return Err(QueueError::Disconnected);
      },
    }
    Ok(())
  })
  .into_dyn(|f| f as &DispatchFn);

  let drop_hook = ArcShared::new(move || {
    drop_state.responder_dropped();
  })
  .into_dyn(|f| f as &DropHookFn);

  let internal = InternalMessageSender::<Mode>::with_drop_hook(dispatch, drop_hook);
  let responder = MessageSender::new(internal);
  (future, responder)
}
