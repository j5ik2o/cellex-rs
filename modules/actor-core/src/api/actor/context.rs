use crate::api::actor::ask::{ask_with_timeout, create_ask_handles, AskError, AskFuture, AskResult, AskTimeoutFuture};
use crate::api::actor::{ActorFailure, ActorRef, Props};
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::api::messaging::{MessageEnvelope, MessageMetadata, MessageSender};
use crate::api::supervision::FailureInfo;
use crate::internal::context::ActorContext;
use crate::{
  ActorId, ActorPath, DynMessage, Extension, ExtensionId, Extensions, MailboxRuntime, MetadataStorageMode,
  RuntimeBound, Supervisor,
};
use alloc::boxed::Box;
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use cellex_utils_core_rs::{Element, QueueError, DEFAULT_PRIORITY};
use core::future::Future;
use core::marker::PhantomData;
use core::time::Duration;

mod context_log_level;
mod context_logger;
mod message_adapter_ref;
mod message_metadata_responder;

pub use context_log_level::ContextLogLevel;
pub use context_logger::ContextLogger;
pub use message_adapter_ref::MessageAdapterRef;
pub use message_metadata_responder::MessageMetadataResponder;

#[cfg(target_has_atomic = "ptr")]
pub(super) type AdapterFn<Ext, U> = dyn Fn(Ext) -> U + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(super) type AdapterFn<Ext, U> = dyn Fn(Ext) -> U;

/// Typed actor execution context wrapper.
/// 'r: lifetime of the mutable reference to ActorContext
/// 'ctx: lifetime parameter of ActorContext itself
pub struct Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode, {
  pub(super) inner: &'r mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>,
  pub(super) metadata: Option<crate::MessageMetadata<MailboxConcurrencyOf<R>>>,
  pub(super) extensions: Extensions,
  pub(super) _marker: PhantomData<U>,
}

/// Type alias for context during setup.
pub type SetupContext<'ctx, U, R> = Context<'ctx, 'ctx, U, R>;

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  pub(super) fn new(inner: &'r mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>) -> Self {
    let extensions = inner.extensions();
    Self {
      inner,
      metadata: None,
      extensions,
      _marker: PhantomData,
    }
  }
}

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Determines if receive timeout is supported.
  #[must_use]
  pub fn has_receive_timeout_support(&self) -> bool {
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

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Gets the metadata accompanying the current message.
  #[must_use]
  pub fn message_metadata(&self) -> Option<&MessageMetadata<MailboxConcurrencyOf<R>>> {
    self.metadata.as_ref()
  }

  pub(crate) fn with_metadata(
    inner: &'r mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>,
    metadata: MessageMetadata<MailboxConcurrencyOf<R>>,
  ) -> Self {
    let extensions = inner.extensions();
    Self {
      inner,
      metadata: Some(metadata),
      extensions,
      _marker: PhantomData,
    }
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
  pub fn inner(&mut self) -> &mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>> {
    self.inner
  }
}

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Spawns a child actor and returns an `ActorRef`.
  pub fn spawn_child<V>(&mut self, props: Props<V, R>) -> ActorRef<V, R>
  where
    V: Element, {
    let (internal_props, supervisor_cfg) = props.into_parts();
    let actor_ref = self
      .inner
      .spawn_child_from_props(Box::new(supervisor_cfg.as_supervisor::<DynMessage>()), internal_props);
    ActorRef::new(actor_ref)
  }
}

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
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
    let adapter = ArcShared::new(f).into_dyn(|func| func as &AdapterFn<Ext, U>);
    MessageAdapterRef::new(self.self_ref(), adapter)
  }

  pub(crate) fn self_dispatcher(&self) -> MessageSender<U, MailboxConcurrencyOf<R>>
  where
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
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
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_sender(self.self_dispatcher());
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
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_sender(sender.to_dispatcher());
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
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let metadata = self.message_metadata().cloned().ok_or(AskError::MissingResponder)?;
    metadata.respond_with(self, message)
  }

  /// Sends an inquiry to the target actor and returns a Future that waits for a response.
  pub fn ask<V, Resp, F>(&mut self, target: &ActorRef<V, R>, factory: F) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element,
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<R>>) -> V,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new()
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
    F: FnOnce(MessageSender<Resp, MailboxConcurrencyOf<R>>) -> V,
    TFut: Future<Output = ()> + Unpin,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let responder_for_message = MessageSender::new(responder.internal());
    let message = factory(responder_for_message);
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new()
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
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new()
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
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
    MailboxSignalOf<R>: Clone + RuntimeBound + 'static, {
    let timeout_future = timeout;
    let (future, responder) = create_ask_handles::<Resp, MailboxConcurrencyOf<R>>();
    let metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new()
      .with_sender(self.self_dispatcher())
      .with_responder(responder);
    match target.tell_with_metadata(message, metadata) {
      Ok(()) => Ok(ask_with_timeout(future, timeout_future)),
      Err(err) => Err(AskError::from(err)),
    }
  }
}
