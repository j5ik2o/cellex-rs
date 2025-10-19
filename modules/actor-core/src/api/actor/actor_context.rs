use alloc::boxed::Box;
use core::{future::Future, marker::PhantomData, time::Duration};

use cellex_utils_core_rs::{
  sync::{ArcShared, SharedBound},
  Element, QueueError, DEFAULT_PRIORITY,
};
use spin::RwLock;

use crate::{
  api::{
    actor::{
      actor_failure::ActorFailure,
      actor_ref::{ActorRef, PriorityActorRef},
      ask::{ask_with_timeout, create_ask_handles, AskError, AskFuture, AskResult, AskTimeoutFuture},
      props::Props,
    },
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    extensions::{Extension, ExtensionId, Extensions},
    mailbox::{
      messages::{PriorityEnvelope, SystemMessage},
      MailboxFactory,
    },
    messaging::{AnyMessage, MessageEnvelope, MessageMetadata, MessageSender, MetadataStorageMode},
    process::{pid::Pid, process_registry::ProcessRegistry},
    supervision::failure::FailureInfo,
  },
  RuntimeBound,
};

mod context_log_level;
mod context_logger;

pub use context_log_level::ContextLogLevel;
pub use context_logger::ContextLogger;

pub use crate::api::actor::{
  message_adapter_ref::MessageAdapterRef, message_metadata_responder::MessageMetadataResponder,
};
use crate::{
  api::actor::{actor_id::ActorId, actor_path::ActorPath},
  internal::actor_context::InternalActorContext,
};

#[cfg(target_has_atomic = "ptr")]
pub(super) type AdapterFn<Ext, U> = dyn Fn(Ext) -> U + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(super) type AdapterFn<Ext, U> = dyn Fn(Ext) -> U;

/// Typed actor execution context wrapper.
/// 'r: lifetime of the mutable reference to ActorContext
/// 'ctx: lifetime parameter of ActorContext itself
pub struct ActorContext<'r, 'ctx, U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  pub(super) inner:      &'r mut InternalActorContext<'ctx, MailboxOf<AR>>,
  pub(super) metadata:   Option<MessageMetadata<MailboxConcurrencyOf<AR>>>,
  pub(super) extensions: Extensions,
  pub(super) _marker:    PhantomData<U>,
}

/// Type alias for context during setup.
pub type SetupContext<'ctx, U, R> = ActorContext<'ctx, 'ctx, U, R>;

impl<'r, 'ctx, U, AR> ActorContext<'r, 'ctx, U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  pub(crate) fn new(inner: &'r mut InternalActorContext<'ctx, MailboxOf<AR>>) -> Self {
    let extensions = inner.extensions();
    Self { inner, metadata: None, extensions, _marker: PhantomData }
  }
}

impl<'r, 'ctx, U, AR> ActorContext<'r, 'ctx, U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Determines if receive timeout is supported.
  #[must_use]
  pub const fn has_receive_timeout_support(&self) -> bool {
    self.inner.has_receive_timeout_scheduler()
  }

  /// Sets the receive timeout.
  pub fn set_receive_timeout(&mut self, duration: Duration) -> bool {
    self.inner.set_receive_timeout(duration)
  }

  /// Cancels the receive timeout.
  pub fn cancel_receive_timeout(&mut self) -> bool {
    self.inner.cancel_receive_timeout()
  }
}

impl<'r, 'ctx, U, AR> ActorContext<'r, 'ctx, U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Gets the metadata accompanying the current message.
  #[must_use]
  pub const fn message_metadata(&self) -> Option<&MessageMetadata<MailboxConcurrencyOf<AR>>> {
    self.metadata.as_ref()
  }

  pub(crate) fn with_metadata(
    inner: &'r mut InternalActorContext<'ctx, MailboxOf<AR>>,
    metadata: MessageMetadata<MailboxConcurrencyOf<AR>>,
  ) -> Self {
    let extensions = inner.extensions();
    Self { inner, metadata: Some(metadata), extensions, _marker: PhantomData }
  }

  /// Returns the shared extension registry.
  #[must_use]
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Applies the provided closure to the extension identified by `id`.
  pub fn extension<E, F, T>(&self, id: ExtensionId, f: F) -> Option<T>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> T, {
    self.extensions.with::<E, _, _>(id, f)
  }

  /// Gets the actor ID of this actor.
  #[must_use]
  pub fn actor_id(&self) -> ActorId {
    self.inner.actor_id()
  }

  /// Gets the actor path of this actor.
  #[must_use]
  pub fn actor_path(&self) -> &ActorPath {
    self.inner.actor_path()
  }

  /// Gets the list of actor IDs watching this actor.
  #[must_use]
  pub fn watchers(&self) -> &[ActorId] {
    self.inner.watchers()
  }

  /// Returns the current processing priority if the actor is executing within a priority context.
  #[must_use]
  pub const fn current_priority(&self) -> Option<i8> {
    self.inner.current_priority()
  }

  /// Gets the PID representing this actor.
  #[must_use]
  pub fn self_pid(&self) -> &Pid {
    self.inner.pid()
  }

  /// Returns the process registry handle for PID resolution.
  #[must_use]
  pub fn process_registry(
    &self,
  ) -> ArcShared<ProcessRegistry<PriorityActorRef<AnyMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<AnyMessage>>>>
  {
    self.inner.process_registry()
  }

  /// Gets the logger for this actor.
  #[must_use]
  pub fn log(&self) -> ContextLogger {
    ContextLogger::new(self.actor_id(), self.actor_path())
  }

  /// Registers a watcher.
  pub fn register_watcher(&mut self, watcher: ActorId) {
    self.inner.register_watcher(watcher);
  }

  /// Unregisters a watcher.
  pub fn unregister_watcher(&mut self, watcher: ActorId) {
    self.inner.unregister_watcher(watcher);
  }

  /// Gets a mutable reference to the internal context.
  pub const fn inner(&mut self) -> &mut InternalActorContext<'ctx, MailboxOf<AR>> {
    self.inner
  }
}

impl<'r, 'ctx, U, AR> ActorContext<'r, 'ctx, U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Spawns a child actor and returns an `ActorRef`.
  pub fn spawn_child<V>(&mut self, props: Props<V, AR>) -> ActorRef<V, AR>
  where
    V: Element, {
    let (internal_props, supervisor_cfg): (crate::internal::actor::InternalProps<MailboxOf<AR>>, _) =
      props.into_parts();
    let pid_slot = ArcShared::new(RwLock::new(None));
    let registry = self.process_registry();
    let actor_ref = self.inner.spawn_child_from_props(
      Box::new(supervisor_cfg.as_supervisor::<AnyMessage>()),
      internal_props,
      pid_slot.clone(),
    );
    ActorRef::new(actor_ref, pid_slot, Some(registry))
  }
}

impl<'r, 'ctx, U, AR> ActorContext<'r, 'ctx, U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Sends a message to itself.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the mailbox refuses the message.
  pub fn send_to_self(&self, message: U) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let dyn_message = AnyMessage::new(MessageEnvelope::user(message));
    self.inner.send_to_self_with_priority(dyn_message, DEFAULT_PRIORITY)
  }

  /// Sends a system message to itself.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the mailbox refuses the system message.
  pub fn send_system_to_self(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let envelope = PriorityEnvelope::from_system(message).map(|sys| AnyMessage::new(MessageEnvelope::<U>::System(sys)));
    self.inner.send_envelope_to_self(envelope)
  }

  /// Reports a failure to the guardian using the supervision channel.
  ///
  /// # Errors
  /// Returns [`QueueError`] when delivering the escalation fails.
  pub fn fail<E>(&self, error: E) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    E: core::fmt::Display + core::fmt::Debug + Send + 'static, {
    let failure = ActorFailure::from_error(error);
    let info = FailureInfo::from_failure(self.actor_id(), self.actor_path().clone(), failure);
    self.send_system_to_self(SystemMessage::Escalate(info))
  }

  /// Gets a reference to itself.
  #[must_use]
  pub fn self_ref(&self) -> ActorRef<U, AR> {
    let registry = self.process_registry();
    let pid_slot = ArcShared::new(RwLock::new(Some(self.self_pid().clone())));
    ActorRef::new(self.inner.self_ref(), pid_slot, Some(registry))
  }

  /// Creates an adapter that converts external message types to internal message types.
  pub fn message_adapter<Ext, F>(&self, f: F) -> MessageAdapterRef<Ext, U, AR>
  where
    Ext: Element,
    F: Fn(Ext) -> U + SharedBound + 'static, {
    let adapter = ArcShared::new(f).into_dyn(|func| func as &AdapterFn<Ext, U>);
    MessageAdapterRef::new(self.self_ref(), adapter)
  }

  pub(crate) fn self_dispatcher(&self) -> MessageSender<U, MailboxConcurrencyOf<AR>>
  where
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    self.self_ref().to_dispatcher()
  }

  /// Requests a message with sender information.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the target mailbox refuses the message.
  pub fn request<V>(
    &mut self,
    target: &ActorRef<V, AR>,
    message: V,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    V: Element,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
      .with_sender(self.self_dispatcher())
      .with_sender_pid(self.self_pid().clone());
    target.tell_with_metadata(message, metadata)
  }

  /// Requests a message with specified sender information.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the target mailbox refuses the message.
  pub fn request_with_sender<V, S>(
    &mut self,
    target: &ActorRef<V, AR>,
    message: V,
    sender: &ActorRef<S, AR>,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    V: Element,
    S: Element,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_sender(sender.to_dispatcher());
    target.tell_with_metadata(message, metadata)
  }

  /// Forwards a message while preserving the original metadata.
  ///
  /// # Errors
  /// Returns [`QueueError`] when the target mailbox refuses the forwarded message.
  pub fn forward<V>(
    &mut self,
    target: &ActorRef<V, AR>,
    message: V,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    V: Element, {
    let metadata = self.message_metadata().cloned().unwrap_or_default();
    target.tell_with_metadata(message, metadata)
  }

  /// Responds to the sender of the current message.
  ///
  /// # Errors
  /// Returns [`AskError`] when no responder is available or delivery fails.
  pub fn respond<Resp>(&mut self, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let metadata = self.message_metadata().cloned().ok_or(AskError::MissingResponder)?;
    metadata.respond_with(self, message)
  }

  /// Sends an inquiry to the target actor and returns a Future that waits for a response.
  ///
  /// # Errors
  /// Returns [`AskError`] when the target mailbox refuses the message.
  pub fn ask<V, Resp, F>(&mut self, target: &ActorRef<V, AR>, factory: F) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<AR>>) -> V,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
      .with_sender(self.self_dispatcher())
      .with_sender_pid(self.self_pid().clone())
      .with_responder(responder)
      .with_responder_pid(self.self_pid().clone());
    match target.tell_with_metadata(message, metadata) {
      | Ok(()) => Ok(future),
      | Err(err) => Err(AskError::from(err)),
    }
  }

  /// Sends an inquiry with timeout and returns a Future that waits for a response.
  ///
  /// # Errors
  /// Returns [`AskError`] when the target mailbox refuses the message.
  pub fn ask_with_timeout<V, Resp, F, TFut>(
    &mut self,
    target: &ActorRef<V, AR>,
    factory: F,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    V: Element,
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<AR>>) -> V,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
      .with_sender(self.self_dispatcher())
      .with_sender_pid(self.self_pid().clone())
      .with_responder(responder)
      .with_responder_pid(self.self_pid().clone());
    match target.tell_with_metadata(message, metadata) {
      | Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      | Err(err) => Err(AskError::from(err)),
    }
  }

  /// Sends an inquiry to the target actor and returns a Future that waits for a response.
  ///
  /// # Errors
  /// Returns [`AskError`] when the target mailbox refuses the message.
  pub fn request_future<V, Resp>(&mut self, target: &ActorRef<V, AR>, message: V) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
      .with_sender(self.self_dispatcher())
      .with_sender_pid(self.self_pid().clone())
      .with_responder(responder)
      .with_responder_pid(self.self_pid().clone());
    target.tell_with_metadata(message, metadata)?;
    Ok(future)
  }

  /// Sends an inquiry with timeout and returns a Future that waits for a response.
  ///
  /// # Errors
  /// Returns [`AskError`] when the target mailbox refuses the message.
  pub fn request_future_with_timeout<V, Resp, TFut>(
    &mut self,
    target: &ActorRef<V, AR>,
    message: V,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    V: Element,
    Resp: Element,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<AR>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<AR>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
      .with_sender(self.self_dispatcher())
      .with_sender_pid(self.self_pid().clone())
      .with_responder(responder)
      .with_responder_pid(self.self_pid().clone());
    match target.tell_with_metadata(message, metadata) {
      | Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      | Err(err) => Err(AskError::from(err)),
    }
  }
}
