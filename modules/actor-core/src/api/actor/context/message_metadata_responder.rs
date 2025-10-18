use cellex_utils_core_rs::{Element, Shared};

use super::Context;
use crate::{
  api::{
    actor::ask::{AskError, AskResult},
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    mailbox::{MailboxFactory, PriorityEnvelope},
    messaging::{DynMessage, MessageEnvelope, MessageMetadata, MetadataStorageMode},
    process::{
      dead_letter::{DeadLetter, DeadLetterReason},
      process_registry::ProcessResolution,
    },
  },
  RuntimeBound,
};

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
    if let Some(dispatcher) = self.dispatcher_for::<Resp>() {
      let dispatch_metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
        .with_sender(ctx.self_dispatcher())
        .with_sender_pid(ctx.self_pid().clone());
      let envelope = MessageEnvelope::user_with_metadata(message, dispatch_metadata);
      return dispatcher.dispatch_envelope(envelope).map_err(AskError::from);
    }

    let target_pid = self.responder_pid().or_else(|| self.sender_pid()).cloned().ok_or(AskError::MissingResponder)?;
    let dispatch_metadata = MessageMetadata::<MailboxConcurrencyOf<AR>>::new()
      .with_sender(ctx.self_dispatcher())
      .with_sender_pid(ctx.self_pid().clone());
    let envelope = MessageEnvelope::user_with_metadata(message, dispatch_metadata);
    respond_via_pid(ctx, target_pid, envelope)
  }
}

fn respond_via_pid<'r, 'ctx, Resp, U, AR>(
  ctx: &mut Context<'r, 'ctx, U, AR>,
  pid: crate::api::process::pid::Pid,
  envelope: MessageEnvelope<Resp>,
) -> AskResult<()>
where
  Resp: Element,
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  let registry = ctx.process_registry();
  match registry.with_ref(|registry| registry.resolve_pid(&pid)) {
    | ProcessResolution::Local(handle) => {
      let dyn_message = DynMessage::new(envelope);
      let priority_envelope = PriorityEnvelope::with_default_priority(dyn_message);
      let send_result = handle.with_ref(|actor_ref| actor_ref.clone()).try_send_envelope(priority_envelope);
      match send_result {
        | Ok(()) => Ok(()),
        | Err(err) => Err(AskError::from(err)),
      }
    },
    | ProcessResolution::Remote => Err(AskError::MissingResponder),
    | ProcessResolution::Unresolved => {
      let dyn_message = DynMessage::new(envelope);
      let priority_envelope = PriorityEnvelope::with_default_priority(dyn_message);
      registry.with_ref(|registry| {
        registry.publish_dead_letter(DeadLetter::new(
          pid.clone(),
          priority_envelope,
          DeadLetterReason::UnregisteredPid,
        ));
      });
      Err(AskError::MissingResponder)
    },
  }
}
