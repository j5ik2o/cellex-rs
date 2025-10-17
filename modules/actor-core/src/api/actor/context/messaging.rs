use super::{Context, MessageAdapterRef, MessageMetadataResponder};
use crate::api::actor::ask::{ask_with_timeout, create_ask_handles, AskError, AskFuture, AskResult, AskTimeoutFuture};
use crate::api::actor::{ActorFailure, ActorRef};
use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::api::messaging::{MessageEnvelope, MessageMetadata, MessageSender};
use crate::api::supervision::FailureInfo;
use crate::MailboxRuntime;
use crate::{DynMessage, RuntimeBound};
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use cellex_utils_core_rs::{Element, QueueError, DEFAULT_PRIORITY};
use core::future::Future;

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: crate::api::actor_runtime::ActorRuntime + 'static,
  crate::api::actor_runtime::MailboxOf<R>: MailboxRuntime + Clone + 'static,
  crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  crate::api::actor_runtime::MailboxSignalOf<R>: Clone,
  crate::api::actor_runtime::MailboxConcurrencyOf<R>: crate::MetadataStorageMode,
{
  /// Sends a message to itself.
  pub fn send_to_self(&self, message: U) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let dyn_message = DynMessage::new(MessageEnvelope::user(message));
    self.inner.send_to_self_with_priority(dyn_message, DEFAULT_PRIORITY)
  }

  /// Sends a system message to itself.
  pub fn send_system_to_self(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let envelope = PriorityEnvelope::from_system(message).map(|sys| DynMessage::new(MessageEnvelope::<U>::System(sys)));
    self.inner.send_envelope_to_self(envelope)
  }

  /// Reports a failure to the guardian using the supervision channel.
  pub fn fail<E>(&self, error: E) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    E: core::fmt::Display + core::fmt::Debug + Send + 'static, {
    let failure = ActorFailure::from_error(error);
    let info = FailureInfo::from_failure(self.actor_id(), self.actor_path().clone(), failure);
    self.send_system_to_self(SystemMessage::Escalate(info))
  }

  /// Gets a reference to itself.
  #[must_use]
  pub fn self_ref(&self) -> ActorRef<U, R> {
    ActorRef::new(self.inner.self_ref())
  }

  /// Creates an adapter that converts external message types to internal message types.
  pub fn message_adapter<Ext, F>(&self, f: F) -> MessageAdapterRef<Ext, U, R>
  where
    Ext: Element,
    F: Fn(Ext) -> U + SharedBound + 'static, {
    let adapter = ArcShared::new(f).into_dyn(|func| func as &super::AdapterFn<Ext, U>);
    MessageAdapterRef::new(self.self_ref(), adapter)
  }

  pub(crate) fn self_dispatcher(&self) -> MessageSender<U, crate::api::actor_runtime::MailboxConcurrencyOf<R>>
  where
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    self.self_ref().to_dispatcher()
  }

  /// Requests a message with sender information.
  pub fn request<V>(
    &mut self,
    target: &ActorRef<V, R>,
    message: V,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    V: Element,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let metadata =
      MessageMetadata::<crate::api::actor_runtime::MailboxConcurrencyOf<R>>::new().with_sender(self.self_dispatcher());
    target.tell_with_metadata(message, metadata)
  }

  /// Requests a message with specified sender information.
  pub fn request_with_sender<V, S>(
    &mut self,
    target: &ActorRef<V, R>,
    message: V,
    sender: &ActorRef<S, R>,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    V: Element,
    S: Element,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let metadata =
      MessageMetadata::<crate::api::actor_runtime::MailboxConcurrencyOf<R>>::new().with_sender(sender.to_dispatcher());
    target.tell_with_metadata(message, metadata)
  }

  /// Forwards a message while preserving the original metadata.
  pub fn forward<V>(
    &mut self,
    target: &ActorRef<V, R>,
    message: V,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    V: Element, {
    let metadata = self.message_metadata().cloned().unwrap_or_default();
    target.tell_with_metadata(message, metadata)
  }

  /// Responds to the sender of the current message.
  pub fn respond<Resp>(&mut self, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let metadata = self.message_metadata().cloned().ok_or(AskError::MissingResponder)?;
    metadata.respond_with(self, message)
  }

  /// Sends an inquiry to the target actor and returns a Future that waits for a response.
  pub fn ask<V, Resp, F>(&mut self, target: &ActorRef<V, R>, factory: F) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element,
    F: FnOnce(MessageSender<Resp, crate::api::actor_runtime::MailboxConcurrencyOf<R>>) -> V,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let (future, responder) = create_ask_handles::<Resp, crate::api::actor_runtime::MailboxConcurrencyOf<R>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<crate::api::actor_runtime::MailboxConcurrencyOf<R>>::new()
      .with_sender(self.self_dispatcher())
      .with_responder(responder);
    match target.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(future),
      Err(err) => Err(AskError::from(err)),
    }
  }

  /// Sends an inquiry with timeout and returns a Future that waits for a response.
  pub fn ask_with_timeout<V, Resp, F, TFut>(
    &mut self,
    target: &ActorRef<V, R>,
    factory: F,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    V: Element,
    Resp: Element,
    F: FnOnce(MessageSender<Resp, crate::api::actor_runtime::MailboxConcurrencyOf<R>>) -> V,
    TFut: Future<Output = ()> + Unpin,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, crate::api::actor_runtime::MailboxConcurrencyOf<R>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<crate::api::actor_runtime::MailboxConcurrencyOf<R>>::new()
      .with_sender(self.self_dispatcher())
      .with_responder(responder);
    match target.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      Err(err) => Err(AskError::from(err)),
    }
  }

  /// Sends an inquiry to the target actor and returns a Future that waits for a response.
  pub fn request_future<V, Resp>(&mut self, target: &ActorRef<V, R>, message: V) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let (future, responder) = create_ask_handles::<Resp, crate::api::actor_runtime::MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<crate::api::actor_runtime::MailboxConcurrencyOf<R>>::new()
      .with_sender(self.self_dispatcher())
      .with_responder(responder);
    target.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Sends an inquiry with timeout and returns a Future that waits for a response.
  pub fn request_future_with_timeout<V, Resp, TFut>(
    &mut self,
    target: &ActorRef<V, R>,
    message: V,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    V: Element,
    Resp: Element,
    TFut: Future<Output = ()> + Unpin,
    crate::api::actor_runtime::MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    crate::api::actor_runtime::MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, crate::api::actor_runtime::MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<crate::api::actor_runtime::MailboxConcurrencyOf<R>>::new()
      .with_sender(self.self_dispatcher())
      .with_responder(responder);
    match target.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      Err(err) => Err(AskError::from(err)),
    }
  }
}
