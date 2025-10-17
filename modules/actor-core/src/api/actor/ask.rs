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

use crate::api::mailbox::MailboxConcurrency;
use crate::internal::message::discard_metadata;
use crate::{DynMessage, InternalMessageSender, MessageEnvelope, MessageSender};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};
use core::future::Future;

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
  .into_dyn(|f| f as &DispatchFn);

  let drop_hook = ArcShared::new(move || {
    drop_state.responder_dropped();
  })
  .into_dyn(|f| f as &DropHookFn);

  let internal = InternalMessageSender::<C>::with_drop_hook(dispatch, drop_hook);
  let responder = MessageSender::new(internal);
  (future, responder)
}
