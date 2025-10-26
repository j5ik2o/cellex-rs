use alloc::{borrow::ToOwned, boxed::Box};
use core::{future::Future, marker::PhantomData};

use cellex_utils_core_rs::{collections::Element, sync::ArcShared, v2::collections::queue::backend::QueueError};
use spin::RwLock;

use super::ask::{ask_with_timeout, AskFuture, AskResult, AskTimeoutFuture};
use crate::{
  api::{
    actor::{actor_ref::ActorRef, props::Props, ChildNaming, SpawnError},
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    extensions::{Extension, ExtensionId, Extensions},
    messaging::MetadataStorageMode,
  },
  internal::actor_system::InternalRootContext,
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Context for operating root actors.
///
/// Performs actor spawning and message sending from the top level of the actor system.
/// Manages failure handling of child actors through guardian strategies.
pub struct RootContext<'a, U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>, {
  pub(crate) inner:   InternalRootContext<'a, AR, Strat>,
  pub(crate) _marker: PhantomData<U>,
}

impl<'a, U, AR, Strat> RootContext<'a, U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>,
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
  pub fn spawn(&mut self, props: Props<U, AR>) -> Result<ActorRef<U, AR>, SpawnError<AnyMessage>>
  where
    AnyMessage: Element, {
    let (internal_props, supervisor_cfg): (crate::internal::actor::InternalProps<MailboxOf<AR>>, _) =
      props.into_parts();
    let pid_slot = ArcShared::new(RwLock::new(None));
    let registry = self.inner.process_registry();
    let actor_ref = self.inner.spawn_with_supervisor(
      Box::new(supervisor_cfg.as_supervisor::<AnyMessage>()),
      internal_props,
      ChildNaming::Auto,
      pid_slot.clone(),
    )?;
    Ok(ActorRef::new(actor_ref, pid_slot, Some(registry)))
  }

  /// Spawns a new actor with a unique name generated from the provided prefix.
  ///
  /// The actual name will be `{prefix}-{n}` where `n` is a monotonically increasing counter that
  /// is guaranteed to be unique within the parent.
  ///
  /// # Errors
  ///
  /// Propagates queue failures from the scheduler.
  pub fn spawn_prefix(&mut self, props: Props<U, AR>, prefix: &str) -> Result<ActorRef<U, AR>, SpawnError<AnyMessage>>
  where
    AnyMessage: Element, {
    let (internal_props, supervisor_cfg): (crate::internal::actor::InternalProps<MailboxOf<AR>>, _) =
      props.into_parts();
    let pid_slot = ArcShared::new(RwLock::new(None));
    let registry = self.inner.process_registry();
    let actor_ref = self.inner.spawn_with_supervisor(
      Box::new(supervisor_cfg.as_supervisor::<AnyMessage>()),
      internal_props,
      ChildNaming::WithPrefix(prefix.to_owned()),
      pid_slot.clone(),
    )?;
    Ok(ActorRef::new(actor_ref, pid_slot, Some(registry)))
  }

  /// Spawns a new actor using the specified name. Fails if the name already exists.
  ///
  /// # Errors
  ///
  /// Returns [`SpawnError::NameExists`] if an actor with the same name already exists, or
  /// [`SpawnError::Queue`] if the scheduler reports a queue failure.
  pub fn spawn_named(&mut self, props: Props<U, AR>, name: &str) -> Result<ActorRef<U, AR>, SpawnError<AnyMessage>>
  where
    AnyMessage: Element, {
    let (internal_props, supervisor_cfg): (crate::internal::actor::InternalProps<MailboxOf<AR>>, _) =
      props.into_parts();
    let pid_slot = ArcShared::new(RwLock::new(None));
    let registry = self.inner.process_registry();
    let actor_ref = self.inner.spawn_with_supervisor(
      Box::new(supervisor_cfg.as_supervisor::<AnyMessage>()),
      internal_props,
      ChildNaming::Explicit(name.to_owned()),
      pid_slot.clone(),
    )?;
    Ok(ActorRef::new(actor_ref, pid_slot, Some(registry)))
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
  ///
  /// # Errors
  /// Returns [`AskError`](crate::api::actor::ask::AskError) when sending the request fails.
  pub fn request_future<V, Resp>(&self, target: &ActorRef<V, AR>, message: V) -> AskResult<AskFuture<Resp>>
  where
    V: Element,
    Resp: Element, {
    target.request_future(message)
  }

  /// Sends a message to the specified actor and returns a Future that waits for a response with
  /// timeout.
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
  ///
  /// # Errors
  /// Returns [`AskError`](crate::api::actor::ask::AskError) when sending the request fails.
  pub fn request_future_with_timeout<V, Resp, TFut>(
    &self,
    target: &ActorRef<V, AR>,
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
  /// # Errors
  /// Returns [`QueueError`] when the underlying scheduler reports a mailbox failure.
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    #[allow(deprecated)]
    self.inner.dispatch_all()
  }

  /// Dispatches one next message.
  ///
  /// # Returns
  ///
  /// `Ok(())` on success, `Err` if a mailbox error occurs
  /// # Errors
  /// Returns [`QueueError`] when the underlying scheduler reports a mailbox failure.
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.inner.dispatch_next().await
  }

  /// Returns the extension registry associated with the actor system.
  #[must_use]
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
