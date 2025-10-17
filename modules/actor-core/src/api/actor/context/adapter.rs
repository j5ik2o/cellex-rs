use super::{AdapterFn, Context};
use crate::api::actor::ask::{AskError, AskResult};
use crate::api::actor::ActorRef;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::{MessageEnvelope, MessageMetadata};
use crate::MailboxRuntime;
use crate::{DynMessage, MetadataStorageMode, RuntimeBound};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Reference to a message adapter.
#[derive(Clone)]
pub struct MessageAdapterRef<Ext, U, R>
where
  Ext: Element,
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  target: ActorRef<U, R>,
  adapter: ArcShared<AdapterFn<Ext, U>>,
}

impl<Ext, U, R> MessageAdapterRef<Ext, U, R>
where
  Ext: Element,
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
{
  pub(crate) fn new(target: ActorRef<U, R>, adapter: ArcShared<AdapterFn<Ext, U>>) -> Self {
    Self { target, adapter }
  }

  /// Converts an external message and sends it to the target actor.
  pub fn tell(&self, message: Ext) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let mapped = (self.adapter)(message);
    self.target.tell(mapped)
  }

  /// Converts an external message and sends it to the target actor with the specified priority.
  pub fn tell_with_priority(&self, message: Ext, priority: i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let mapped = (self.adapter)(message);
    self.target.tell_with_priority(mapped, priority)
  }

  /// Gets a reference to the target actor.
  #[must_use]
  pub fn target(&self) -> &ActorRef<U, R> {
    &self.target
  }
}

pub trait MessageMetadataResponder<R>
where
  R: ActorRuntime,
  MailboxOf<R>: MailboxRuntime + Clone + 'static, {
  fn respond_with<Resp, U>(&self, ctx: &mut Context<'_, '_, U, R>, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    U: Element;
}

impl<R> MessageMetadataResponder<R> for MessageMetadata<MailboxConcurrencyOf<R>>
where
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
  MailboxSignalOf<R>: Clone + RuntimeBound + 'static,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  fn respond_with<Resp, U>(&self, ctx: &mut Context<'_, '_, U, R>, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    U: Element, {
    let dispatcher = self.dispatcher_for::<Resp>().ok_or(AskError::MissingResponder)?;
    let dispatch_metadata = MessageMetadata::<MailboxConcurrencyOf<R>>::new().with_sender(ctx.self_dispatcher());
    let envelope = MessageEnvelope::user_with_metadata(message, dispatch_metadata);
    dispatcher.dispatch_envelope(envelope).map_err(AskError::from)
  }
}
