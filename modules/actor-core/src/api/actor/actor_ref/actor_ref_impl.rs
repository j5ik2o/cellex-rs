use core::{future::Future, marker::PhantomData};

use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError, Shared, SharedBound};
use spin::RwLock;

use super::priority_actor_ref::PriorityActorRef;
use crate::{
  api::{
    actor::ask::{ask_with_timeout, create_ask_handles, AskError, AskFuture, AskResult, AskTimeoutFuture},
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    mailbox::{
      messages::{PriorityEnvelope, SystemMessage},
      MailboxFactory,
    },
    messaging::{AnyMessage, MessageEnvelope, MessageMetadata, MessageSender, MetadataStorageMode},
    process::{
      dead_letter::{DeadLetter, DeadLetterReason},
      pid::Pid,
      process_registry::ProcessRegistry,
    },
  },
  internal::message::InternalMessageSender,
};

type ActorProcessRegistry<AR> =
  ProcessRegistry<PriorityActorRef<AnyMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<AnyMessage>>>;
type ActorRegistryShared<AR> = ArcShared<ActorProcessRegistry<AR>>;
type ActorRegistrySharedRef<'a, AR> = Option<&'a ActorRegistryShared<AR>>;

/// Typed actor reference.
///
/// Used to send user messages and system messages to the mailbox,
/// and to receive responses via `ask`-style APIs.
#[derive(Clone)]
pub struct ActorRef<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  inner:            PriorityActorRef<AnyMessage, MailboxOf<AR>>,
  pid_slot:         ArcShared<RwLock<Option<Pid>>>,
  process_registry: Option<ActorRegistryShared<AR>>,
  _marker:          PhantomData<U>,
}

impl<U, AR> ActorRef<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Creates a new `ActorRef` from an internal reference.
  pub(crate) const fn new(
    inner: PriorityActorRef<AnyMessage, MailboxOf<AR>>,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
    process_registry: Option<ActorRegistryShared<AR>>,
  ) -> Self {
    Self { inner, pid_slot, process_registry, _marker: PhantomData }
  }

  /// Creates an `ActorRef` without an associated process registry.
  #[allow(dead_code)]
  pub(crate) fn new_without_registry(inner: PriorityActorRef<AnyMessage, MailboxOf<AR>>) -> Self {
    Self::new(inner, ArcShared::new(RwLock::new(None)), None)
  }

  /// Returns the PID slot associated with this reference.
  #[allow(dead_code)]
  pub(crate) fn pid_slot(&self) -> ArcShared<RwLock<Option<Pid>>> {
    self.pid_slot.clone()
  }

  /// Sets the PID for this reference.
  #[allow(dead_code)]
  pub(crate) fn set_pid(&self, pid: Pid) {
    *self.pid_slot.write() = Some(pid);
  }

  /// Returns the currently known PID.
  fn current_pid(&self) -> Option<Pid> {
    Self::current_pid_from_slot(&self.pid_slot)
  }

  /// Returns the PID associated with this reference when available.
  #[must_use]
  pub fn pid(&self) -> Option<Pid> {
    self.current_pid()
  }

  fn current_pid_from_slot(pid_slot: &ArcShared<RwLock<Option<Pid>>>) -> Option<Pid> {
    pid_slot.read().clone()
  }

  fn take_shared_envelope(shared: ArcShared<PriorityEnvelope<AnyMessage>>) -> Option<PriorityEnvelope<AnyMessage>> {
    shared.try_unwrap().ok()
  }

  /// Wraps a user message into a dynamic message.
  pub(crate) fn wrap_user(message: U) -> AnyMessage {
    AnyMessage::new(MessageEnvelope::user(message))
  }

  /// Wraps a user message with metadata into a dynamic message.
  pub(crate) fn wrap_user_with_metadata(message: U, metadata: MessageMetadata<MailboxConcurrencyOf<AR>>) -> AnyMessage {
    AnyMessage::new(MessageEnvelope::user_with_metadata(message, metadata))
  }

  fn dispatch_envelope_internal(
    &self,
    envelope: PriorityEnvelope<AnyMessage>,
    unresolved_reason: DeadLetterReason,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    Self::dispatch_envelope_with_parts(
      &self.inner,
      &self.pid_slot,
      self.process_registry.as_ref(),
      envelope,
      unresolved_reason,
    )
  }

  fn dispatch_envelope_with_parts(
    inner: &PriorityActorRef<AnyMessage, MailboxOf<AR>>,
    pid_slot: &ArcShared<RwLock<Option<Pid>>>,
    registry: ActorRegistrySharedRef<'_, AR>,
    envelope: PriorityEnvelope<AnyMessage>,
    unresolved_reason: DeadLetterReason,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let pid_opt = Self::current_pid_from_slot(pid_slot);
    if let (Some(registry), Some(pid)) = (registry, pid_opt.as_ref()) {
      let envelope_shared = ArcShared::new(envelope);
      let resolution = registry.with_ref(|registry: &ActorProcessRegistry<AR>| {
        registry.resolve_or_dead_letter_with_remote(
          pid,
          envelope_shared.clone(),
          unresolved_reason,
          DeadLetterReason::NetworkUnreachable,
        )
      });
      let Some(handle) = resolution else {
        return Err(QueueError::Disconnected);
      };
      let Some(envelope) = Self::take_shared_envelope(envelope_shared) else {
        return Err(QueueError::Disconnected);
      };
      let actor_ref = handle.with_ref(|actor_ref: &PriorityActorRef<AnyMessage, MailboxOf<AR>>| actor_ref.clone());
      let send_result = actor_ref.try_send_envelope(envelope);
      return Self::map_send_result(Some(registry), pid_opt.as_ref(), send_result);
    }

    let send_result = inner.try_send_envelope(envelope);
    Self::map_send_result(registry, pid_opt.as_ref(), send_result)
  }

  #[allow(dead_code)]
  fn dispatch_dyn(&self, message: AnyMessage, priority: i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    Self::dispatch_dyn_with_parts(&self.inner, &self.pid_slot, self.process_registry.as_ref(), message, priority)
  }

  fn dispatch_dyn_with_parts(
    inner: &PriorityActorRef<AnyMessage, MailboxOf<AR>>,
    pid_slot: &ArcShared<RwLock<Option<Pid>>>,
    registry: ActorRegistrySharedRef<'_, AR>,
    message: AnyMessage,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let envelope = PriorityEnvelope::new(message, priority);
    Self::dispatch_envelope_with_parts(inner, pid_slot, registry, envelope, DeadLetterReason::UnregisteredPid)
  }

  fn map_send_result(
    registry: ActorRegistrySharedRef<'_, AR>,
    pid: Option<&Pid>,
    result: Result<(), QueueError<PriorityEnvelope<AnyMessage>>>,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    match result {
      | Ok(()) => Ok(()),
      | Err(error) => Err(Self::handle_send_error(registry, pid, error)),
    }
  }

  fn handle_send_error(
    registry: ActorRegistrySharedRef<'_, AR>,
    pid: Option<&Pid>,
    error: QueueError<PriorityEnvelope<AnyMessage>>,
  ) -> QueueError<PriorityEnvelope<AnyMessage>> {
    match error {
      | QueueError::Full(envelope) => {
        if let (Some(registry), Some(pid)) = (registry, pid) {
          let shared = ArcShared::new(envelope);
          registry.with_ref(|registry: &ActorProcessRegistry<AR>| {
            let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::DeliveryRejected);
            registry.publish_dead_letter(&letter);
          });
          match Self::take_shared_envelope(shared) {
            | Some(envelope) => QueueError::Full(envelope),
            | None => QueueError::Disconnected,
          }
        } else {
          QueueError::Full(envelope)
        }
      },
      | QueueError::OfferError(envelope) => {
        if let (Some(registry), Some(pid)) = (registry, pid) {
          let shared = ArcShared::new(envelope);
          registry.with_ref(|registry: &ActorProcessRegistry<AR>| {
            let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::DeliveryRejected);
            registry.publish_dead_letter(&letter);
          });
          match Self::take_shared_envelope(shared) {
            | Some(envelope) => QueueError::OfferError(envelope),
            | None => QueueError::Disconnected,
          }
        } else {
          QueueError::OfferError(envelope)
        }
      },
      | QueueError::Closed(envelope) => {
        if let (Some(registry), Some(pid)) = (registry, pid) {
          let shared = ArcShared::new(envelope);
          registry.with_ref(|registry: &ActorProcessRegistry<AR>| {
            let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::Terminated);
            registry.publish_dead_letter(&letter);
          });
          match Self::take_shared_envelope(shared) {
            | Some(envelope) => QueueError::Closed(envelope),
            | None => QueueError::Disconnected,
          }
        } else {
          QueueError::Closed(envelope)
        }
      },
      | QueueError::Disconnected => QueueError::Disconnected,
    }
  }

  /// Sends a message with metadata (internal API).
  pub(crate) fn tell_with_metadata(
    &self,
    message: U,
    metadata: MessageMetadata<MailboxConcurrencyOf<AR>>,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let dyn_message = Self::wrap_user_with_metadata(message, metadata);
    let envelope = PriorityEnvelope::with_default_priority(dyn_message);
    self.dispatch_envelope_internal(envelope, DeadLetterReason::UnregisteredPid)
  }

  /// Sends a message (Fire-and-Forget).
  ///
  /// # Errors
  /// Returns [`QueueError`] when the underlying mailbox rejects the message.
  pub fn tell(&self, message: U) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let dyn_message = Self::wrap_user(message);
    let envelope = PriorityEnvelope::with_default_priority(dyn_message);
    self.dispatch_envelope_internal(envelope, DeadLetterReason::UnregisteredPid)
  }

  /// Sends a message with specified priority.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the underlying mailbox rejects the message.
  pub fn tell_with_priority(&self, message: U, priority: i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let dyn_message = Self::wrap_user(message);
    let envelope = PriorityEnvelope::new(dyn_message, priority);
    self.dispatch_envelope_internal(envelope, DeadLetterReason::UnregisteredPid)
  }

  /// Sends a system message.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the underlying mailbox rejects the message.
  pub fn send_system(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let envelope = PriorityEnvelope::from_system(message).map(|sys| AnyMessage::new(MessageEnvelope::<U>::System(sys)));
    self.dispatch_envelope_internal(envelope, DeadLetterReason::UnregisteredPid)
  }

  /// Converts this actor reference to a message dispatcher.
  pub fn to_dispatcher(&self) -> MessageSender<U, MailboxConcurrencyOf<AR>>
  where
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + SharedBound + 'static,
    MailboxSignalOf<AR>: Clone + SharedBound + 'static, {
    let inner = self.inner.clone();
    let pid_slot = self.pid_slot.clone();
    let registry = self.process_registry.clone();
    #[cfg(target_has_atomic = "ptr")]
    let dispatch = ArcShared::new(move |message: AnyMessage, priority: i8| {
      ActorRef::<U, AR>::dispatch_dyn_with_parts(&inner, &pid_slot, registry.as_ref(), message, priority)
    })
    .into_dyn(|f| f as &(dyn Fn(AnyMessage, i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + Send + Sync));

    #[cfg(not(target_has_atomic = "ptr"))]
    let dispatch = ArcShared::new(move |message: AnyMessage, priority: i8| {
      ActorRef::<U, AR>::dispatch_dyn_with_parts(&inner, &pid_slot, registry.as_ref(), message, priority)
    })
    .into_dyn(|f| f as &(dyn Fn(AnyMessage, i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>));
    let internal = InternalMessageSender::<MailboxConcurrencyOf<AR>>::new(dispatch);
    MessageSender::new(internal)
  }

  /// Sends a request with specified sender actor (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_from<S>(
    &self,
    message: U,
    sender: &ActorRef<S, AR>,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    S: Element,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + SharedBound + 'static,
    MailboxSignalOf<AR>: Clone + SharedBound + 'static, {
    self.request_with_dispatcher(message, sender.to_dispatcher())
  }

  /// Sends a request with specified dispatcher (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_with_dispatcher<S>(
    &self,
    message: U,
    sender: MessageSender<S, MailboxConcurrencyOf<AR>>,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    S: Element, {
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_sender(sender);
    self.tell_with_metadata(message, metadata)
  }

  /// Generates a response channel internally, sends `message`, and returns `AskFuture` (internal
  /// API).
  ///
  /// # Errors
  /// Returns [`AskError`] when sending the request fails.
  pub(crate) fn request_future<Resp>(&self, message: U) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_responder(responder);
    self.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Issues `ask` with specified sender actor reference (internal API).
  ///
  /// # Errors
  /// Returns [`AskError`] when sending the request fails.
  #[allow(dead_code)]
  pub(crate) fn request_future_from<Resp, S>(
    &self,
    message: U,
    sender: &ActorRef<S, AR>,
  ) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element,
    S: Element,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + SharedBound + 'static,
    MailboxSignalOf<AR>: Clone + SharedBound + 'static, {
    self.request_future_with_dispatcher(message, sender.to_dispatcher())
  }

  /// Issues `ask` with arbitrary dispatcher as sender (internal API).
  ///
  /// # Errors
  /// Returns [`AskError`] when sending the request fails.
  #[allow(dead_code)]
  pub(crate) fn request_future_with_dispatcher<Resp, S>(
    &self,
    message: U,
    sender: MessageSender<S, MailboxConcurrencyOf<AR>>,
  ) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element,
    S: Element, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_sender(sender).with_responder(responder);
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
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + SharedBound + 'static,
    MailboxSignalOf<AR>: Clone + SharedBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_responder(responder);
    match self.tell_with_metadata(message, metadata) {
      | Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      | Err(err) => Err(AskError::from(err)),
    }
  }

  /// Issues `ask` with timeout and specified sender (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_with_timeout_from<Resp, S, TFut>(
    &self,
    message: U,
    sender: &ActorRef<S, AR>,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    S: Element,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + SharedBound + 'static,
    MailboxSignalOf<AR>: Clone + SharedBound + 'static, {
    self.request_future_with_timeout_dispatcher(message, sender.to_dispatcher(), timeout)
  }

  /// Issues `ask` with timeout and specified dispatcher (internal API).
  #[allow(dead_code)]
  pub(crate) fn request_future_with_timeout_dispatcher<Resp, S, TFut>(
    &self,
    message: U,
    sender: MessageSender<S, MailboxConcurrencyOf<AR>>,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    S: Element,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + SharedBound + 'static,
    MailboxSignalOf<AR>: Clone + SharedBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_sender(sender).with_responder(responder);
    match self.tell_with_metadata(message, metadata) {
      | Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      | Err(err) => Err(AskError::from(err)),
    }
  }

  /// Constructs a message using a factory function and sends it with `ask` pattern.
  ///
  /// # Errors
  /// Returns [`AskError`] when sending the request fails.
  pub fn ask_with<Resp, F>(&self, factory: F) -> AskResult<AskFuture<Resp>>
  where
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<AR>>) -> U, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_responder(responder);
    self.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Issues `ask` using a factory function with timeout.
  ///
  /// # Errors
  /// Returns [`AskError`] when sending the request fails.
  pub fn ask_with_timeout<Resp, F, TFut>(&self, factory: F, timeout: TFut) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<AR>>) -> U,
    TFut: Future<Output = ()> + Unpin, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_responder(responder);
    match self.tell_with_metadata(message, metadata) {
      | Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      | Err(err) => Err(AskError::from(err)),
    }
  }
}
