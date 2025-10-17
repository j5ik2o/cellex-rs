use super::priority_actor_ref::PriorityActorRef;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::{DynMessage, MailboxRuntime, MetadataStorageMode, RuntimeBound};
use cellex_utils_core_rs::{Element, QueueError, DEFAULT_PRIORITY};
use core::future::Future;
use core::marker::PhantomData;

use super::super::{ask::create_ask_handles, ask_with_timeout, AskError, AskFuture, AskResult, AskTimeoutFuture};
use crate::api::{InternalMessageSender, MessageEnvelope, MessageMetadata, MessageSender};

/// Typed actor reference.
///
/// Used to send user messages and system messages to the mailbox,
/// and to receive responses via `ask`-style APIs.
#[derive(Clone)]
pub struct ActorRef<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode, {
  inner: PriorityActorRef<DynMessage, MailboxOf<R>>,
  _marker: PhantomData<U>,
}

impl<U, R> ActorRef<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Creates a new `ActorRef` from an internal reference.
  pub(crate) const fn new(inner: PriorityActorRef<DynMessage, MailboxOf<R>>) -> Self {
    Self {
      inner,
      _marker: PhantomData,
    }
  }

  /// Wraps a user message into a dynamic message.
  pub(crate) fn wrap_user(message: U) -> DynMessage {
    DynMessage::new(MessageEnvelope::user(message))
  }

  /// Wraps a user message with metadata into a dynamic message.
  pub(crate) fn wrap_user_with_metadata(message: U, metadata: MessageMetadata<MailboxConcurrencyOf<R>>) -> DynMessage {
    DynMessage::new(MessageEnvelope::user_with_metadata(message, metadata))
  }

  /// Sends an already wrapped message with priority.
  fn send_envelope(
    &self,
    dyn_message: DynMessage,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.try_send_with_priority(dyn_message, priority)
  }

  /// Sends a message with metadata (internal API).
  pub(crate) fn tell_with_metadata(
    &self,
    message: U,
    metadata: MessageMetadata<MailboxConcurrencyOf<R>>,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let dyn_message = Self::wrap_user_with_metadata(message, metadata);
    self.send_envelope(dyn_message, DEFAULT_PRIORITY)
  }

  /// Sends a message (Fire-and-Forget).
  pub fn tell(&self, message: U) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self
      .inner
      .try_send_with_priority(Self::wrap_user(message), DEFAULT_PRIORITY)
  }

  /// Sends a message with specified priority.
  pub fn tell_with_priority(&self, message: U, priority: i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.try_send_with_priority(Self::wrap_user(message), priority)
  }

  /// Sends a system message.
  pub fn send_system(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let envelope = PriorityEnvelope::from_system(message).map(|sys| DynMessage::new(MessageEnvelope::<U>::System(sys)));
    self.inner.try_send_envelope(envelope)
  }

  /// Converts this actor reference to a message dispatcher.
  pub fn to_dispatcher(&self) -> MessageSender<U, MailboxConcurrencyOf<R>>
  where
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let internal = InternalMessageSender::<MailboxConcurrencyOf<R>>::from_factory_ref(self.inner.clone());
    MessageSender::new(internal)
  }

  /// Sends a request with specified sender actor (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_from<S>(
    &self,
    message: U,
    sender: &ActorRef<S, R>,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    S: Element,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    self.request_with_dispatcher(message, sender.to_dispatcher())
  }

  /// Sends a request with specified dispatcher (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_with_dispatcher<S>(
    &self,
    message: U,
    sender: MessageSender<S, MailboxConcurrencyOf<R>>,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    S: Element, {
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_sender(sender);
    self.tell_with_metadata(message, metadata)
  }

  /// Generates a response channel internally, sends `message`, and returns `AskFuture` (internal API).
  pub(crate) fn request_future<Resp>(&self, message: U) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_responder(responder);
    self.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Issues `ask` with specified sender actor reference (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_from<Resp, S>(&self, message: U, sender: &ActorRef<S, R>) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element,
    S: Element,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    self.request_future_with_dispatcher(message, sender.to_dispatcher())
  }

  /// Issues `ask` with arbitrary dispatcher as sender (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_with_dispatcher<Resp, S>(
    &self,
    message: U,
    sender: MessageSender<S, MailboxConcurrencyOf<R>>,
  ) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element,
    S: Element, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new()
      .with_sender(sender)
      .with_responder(responder);
    self.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Issues `ask` with timeout (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_with_timeout<Resp, TFut>(
    &self,
    message: U,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_responder(responder);
    match self.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      Err(err) => Err(AskError::from(err)),
    }
  }

  /// Issues `ask` with timeout and specified sender (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_with_timeout_from<Resp, S, TFut>(
    &self,
    message: U,
    sender: &ActorRef<S, R>,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    S: Element,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    self.request_future_with_timeout_dispatcher(message, sender.to_dispatcher(), timeout)
  }

  /// Issues `ask` with timeout and specified dispatcher (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_with_timeout_dispatcher<Resp, S, TFut>(
    &self,
    message: U,
    sender: MessageSender<S, MailboxConcurrencyOf<R>>,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    S: Element,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new()
      .with_sender(sender)
      .with_responder(responder);
    match self.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      Err(err) => Err(AskError::from(err)),
    }
  }

  /// Constructs a message using a factory function and sends it with `ask` pattern.
  pub fn ask_with<Resp, F>(&self, factory: F) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<R>>) -> U, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_responder(responder);
    self.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Issues `ask` using a factory function with timeout.
  pub fn ask_with_timeout<Resp, F, TFut>(&self, factory: F, timeout: TFut) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<R>>) -> U,
    TFut: Future<Output = ()> + Unpin, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_responder(responder);
    match self.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      Err(err) => Err(AskError::from(err)),
    }
  }
}
