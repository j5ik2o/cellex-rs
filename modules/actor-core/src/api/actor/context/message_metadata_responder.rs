use super::Context;
use crate::api::actor::ask::{AskError, AskResult};
use crate::api::actor_runtime::MailboxOf;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::messaging::MetadataStorageMode;
use crate::api::messaging::{MessageEnvelope, MessageMetadata};
use crate::RuntimeBound;
use cellex_utils_core_rs::Element;

/// Trait allowing message metadata to respond to the original sender.
pub trait MessageMetadataResponder<AR>
where
  AR: ActorRuntime,
  MailboxOf<AR>: MailboxFactory + Clone + 'static, {
  /// Sends a response message back to the original sender.
  fn respond_with<Resp, U>(&self, ctx: &mut Context<'_, '_, U, AR>, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    U: Element;
}

impl<AR> MessageMetadataResponder<AR> for MessageMetadata<MailboxConcurrencyOf<AR>>
where
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone + RuntimeBound + 'static,
  MailboxSignalOf<AR>: Clone + RuntimeBound + 'static,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  fn respond_with<Resp, U>(&self, ctx: &mut Context<'_, '_, U, AR>, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    U: Element, {
    let dispatcher = self.dispatcher_for::<Resp>().ok_or(AskError::MissingResponder)?;
    let dispatch_metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new().with_sender(ctx.self_dispatcher());
    let envelope = MessageEnvelope::user_with_metadata(message, dispatch_metadata);
    dispatcher.dispatch_envelope(envelope).map_err(AskError::from)
  }
}
