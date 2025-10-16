use crate::api::actor::actor_runtime::{
  ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf,
};
use crate::internal::message::{DynMessage, MetadataStorageMode};
use crate::internal::scheduler::{ChildNaming, SpawnError};
use crate::internal::system::InternalRootContext;
use crate::{ActorRef, Extension, ExtensionId, Extensions, PriorityEnvelope, Props};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use cellex_utils_core_rs::{Element, QueueError};
use core::future::Future;
use core::marker::PhantomData;

use super::{ask_with_timeout, AskFuture, AskResult, AskTimeoutFuture};

/// Context for operating root actors.
///
/// Performs actor spawning and message sending from the top level of the actor system.
/// Manages failure handling of child actors through guardian strategies.
pub struct RootContext<'a, U, R, Strat>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, MailboxOf<R>>, {
  pub(crate) inner: InternalRootContext<'a, DynMessage, R, Strat>,
  pub(crate) _marker: PhantomData<U>,
}

impl<'a, U, R, Strat> RootContext<'a, U, R, Strat>
where
  U: Element,
  R: ActorRuntime,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
  Strat: crate::api::guardian::GuardianStrategy<DynMessage, MailboxOf<R>>,
{
  /// Spawns a new actor using the specified properties.
  ///
  /// # Arguments
  ///
  /// * `props` - Properties to use for spawning the actor
  ///
  /// # Returns
  ///
  /// Reference to the spawned actor, or a [`SpawnError`] if the scheduler rejects the spawn.
  ///
  /// # Errors
  ///
  /// Returns [`SpawnError::Queue`] when the underlying scheduler encounters a queue failure.
  pub fn spawn(&mut self, props: Props<U, R>) -> Result<ActorRef<U, R>, SpawnError<DynMessage>>
  where
    DynMessage: Element, {
    let (internal_props, supervisor_cfg) = props.into_parts();
    let actor_ref = self.inner.spawn_with_supervisor(
      Box::new(supervisor_cfg.as_supervisor()),
      internal_props,
      ChildNaming::Auto,
    )?;
    Ok(ActorRef::new(actor_ref))
  }

  /// Spawns a new actor with a unique name generated from the provided prefix.
  ///
  /// The actual name will be `{prefix}-{n}` where `n` is a monotonically increasing counter that
  /// is guaranteed to be unique within the parent.
  ///
  /// # Errors
  ///
  /// Propagates queue failures from the scheduler.
  pub fn spawn_prefix(&mut self, props: Props<U, R>, prefix: &str) -> Result<ActorRef<U, R>, SpawnError<DynMessage>>
  where
    DynMessage: Element, {
    let (internal_props, supervisor_cfg) = props.into_parts();
    let actor_ref = self.inner.spawn_with_supervisor(
      Box::new(supervisor_cfg.as_supervisor()),
      internal_props,
      ChildNaming::WithPrefix(prefix.to_owned()),
    )?;
    Ok(ActorRef::new(actor_ref))
  }

  /// Spawns a new actor using the specified name. Fails if the name already exists.
  ///
  /// # Errors
  ///
  /// Returns [`SpawnError::NameExists`] if an actor with the same name already exists, or
  /// [`SpawnError::Queue`] if the scheduler reports a queue failure.
  pub fn spawn_named(&mut self, props: Props<U, R>, name: &str) -> Result<ActorRef<U, R>, SpawnError<DynMessage>>
  where
    DynMessage: Element, {
    let (internal_props, supervisor_cfg) = props.into_parts();
    let actor_ref = self.inner.spawn_with_supervisor(
      Box::new(supervisor_cfg.as_supervisor()),
      internal_props,
      ChildNaming::Explicit(name.to_owned()),
    )?;
    Ok(ActorRef::new(actor_ref))
  }

  /// Sends a message to the specified actor and returns a Future that waits for a response.
  ///
  /// # Arguments
  ///
  /// * `target` - Target actor to send the message to
  /// * `message` - Message to send
  ///
  /// # Returns
  ///
  /// Future for receiving the response, or an error
  pub fn request_future<V, Resp>(&self, target: &ActorRef<V, R>, message: V) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element, {
    target.request_future(message)
  }

  /// Sends a message to the specified actor and returns a Future that waits for a response with timeout.
  ///
  /// # Arguments
  ///
  /// * `target` - Target actor to send the message to
  /// * `message` - Message to send
  /// * `timeout` - Future indicating timeout
  ///
  /// # Returns
  ///
  /// Future for receiving the response with timeout, or an error
  pub fn request_future_with_timeout<V, Resp, TFut>(
    &self,
    target: &ActorRef<V, R>,
    message: V,
    timeout: TFut,
  ) -> AskResult<AskTimeoutFuture<Resp, TFut>>
  where
    V: Element,
    Resp: Element,
    TFut: Future<Output = ()> + Unpin, {
    let future = target.request_future(message)?;
    Ok(ask_with_timeout(future, timeout))
  }

  /// Dispatches all messages.
  ///
  /// # Returns
  ///
  /// `Ok(())` on success, `Err` if a mailbox error occurs
  ///
  /// # Deprecated
  ///
  /// Deprecated since version 3.1.0. Use `dispatch_next` or `run_until` instead.
  #[deprecated(since = "3.1.0", note = "Use dispatch_next or run_until instead")]
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    #[allow(deprecated)]
    self.inner.dispatch_all()
  }

  /// Dispatches one next message.
  ///
  /// # Returns
  ///
  /// `Ok(())` on success, `Err` if a mailbox error occurs
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.dispatch_next().await
  }

  /// Returns the extension registry associated with the actor system.
  pub fn extensions(&self) -> Extensions {
    self.inner.extensions()
  }

  /// Applies the provided closure to the extension identified by `id`.
  pub fn extension<E, F, T>(&self, id: ExtensionId, f: F) -> Option<T>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> T, {
    self.extensions().with::<E, _, _>(id, f)
  }
}
