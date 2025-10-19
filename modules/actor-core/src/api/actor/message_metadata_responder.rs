use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError, Shared};

use crate::{
  api::{
    actor::{
      actor_context::ActorContext,
      ask::{AskError, AskResult},
    },
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::{AnyMessage, MessageEnvelope, MessageMetadata, MetadataStorageMode},
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
  ///
  /// # Errors
  /// Returns [`AskError`] when no responder can be resolved or delivery fails.
  fn respond_with<Resp, U>(&self, ctx: &mut ActorContext<'_, '_, U, AR>, message: Resp) -> AskResult<()>
  where
    Resp: Element,
    U: Element;
}

impl<AR> MessageMetadataResponder<AR> for MessageMetadata<MailboxConcurrencyOf<AR>>
where
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
  MailboxSignalOf<AR>: Clone + RuntimeBound + 'static,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  fn respond_with<Resp, U>(&self, ctx: &mut ActorContext<'_, '_, U, AR>, message: Resp) -> AskResult<()>
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
  ctx: &mut ActorContext<'r, 'ctx, U, AR>,
  pid: crate::api::process::pid::Pid,
  envelope: MessageEnvelope<Resp>,
) -> AskResult<()>
where
  Resp: Element,
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  let registry = ctx.process_registry();
  match registry.with_ref(|registry| registry.resolve_pid(&pid)) {
    | ProcessResolution::Local(handle) => {
      let dyn_message = AnyMessage::new(envelope);
      let send_result = handle
        .with_ref(|actor_ref| actor_ref.clone())
        .try_send_envelope(PriorityEnvelope::with_default_priority(dyn_message));
      match send_result {
        | Ok(()) => Ok(()),
        | Err(QueueError::Full(envelope)) | Err(QueueError::OfferError(envelope)) => {
          let shared = ArcShared::new(envelope);
          registry.with_ref(|registry| {
            let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::DeliveryRejected);
            registry.publish_dead_letter(&letter);
          });
          let _ = shared.try_unwrap();
          Err(AskError::SendFailed(QueueError::Disconnected))
        },
        | Err(QueueError::Closed(envelope)) => {
          let shared = ArcShared::new(envelope);
          registry.with_ref(|registry| {
            let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::Terminated);
            registry.publish_dead_letter(&letter);
          });
          let _ = shared.try_unwrap();
          Err(AskError::SendFailed(QueueError::Disconnected))
        },
        | Err(QueueError::Disconnected) => Err(AskError::SendFailed(QueueError::Disconnected)),
      }
    },
    | ProcessResolution::Remote => {
      let dyn_message = AnyMessage::new(envelope);
      let priority_envelope = PriorityEnvelope::with_default_priority(dyn_message);
      let shared = ArcShared::new(priority_envelope);
      registry.with_ref(|registry| {
        let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::NetworkUnreachable);
        registry.publish_dead_letter(&letter);
      });
      let _ = shared.try_unwrap();
      Err(AskError::MissingResponder)
    },
    | ProcessResolution::Unresolved => {
      let dyn_message = AnyMessage::new(envelope);
      let priority_envelope = PriorityEnvelope::with_default_priority(dyn_message);
      let shared = ArcShared::new(priority_envelope);
      registry.with_ref(|registry| {
        let letter = DeadLetter::new(pid.clone(), shared.clone(), DeadLetterReason::UnregisteredPid);
        registry.publish_dead_letter(&letter);
      });
      let _ = shared.try_unwrap();
      Err(AskError::MissingResponder)
    },
  }
}
