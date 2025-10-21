use alloc::boxed::Box;

use cellex_utils_core_rs::Element;

use crate::{
  api::{
    actor::{actor_context::ActorContext, actor_failure::ActorFailure, ActorHandlerFn},
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    mailbox::{MailboxFactory, MailboxOptions},
    messaging::MetadataStorageMode,
  },
  internal::actor_context::InternalActorContext,
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MapSystemShared, MessageEnvelope},
  },
};

pub(crate) struct InternalProps<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  pub options:    MailboxOptions,
  pub map_system: MapSystemShared<AnyMessage>,
  pub handler:    Box<ActorHandlerFn<AnyMessage, MF>>,
}

impl<MF> InternalProps<MF>
where
  MF: MailboxFactory + Clone,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  pub fn new(
    options: MailboxOptions,
    map_system: MapSystemShared<AnyMessage>,
    handler: impl for<'ctx> FnMut(&mut InternalActorContext<'ctx, MF>, AnyMessage) -> Result<(), ActorFailure> + 'static,
  ) -> Self {
    Self { options, map_system, handler: Box::new(handler) }
  }
}

pub(crate) fn internal_props_from_adapter<U, AR>(
  options: MailboxOptions,
  map_system: MapSystemShared<AnyMessage>,
  mut adapter: crate::api::actor::behavior::ActorAdapter<U, AR>,
) -> InternalProps<MailboxOf<AR>>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  InternalProps::new(options, map_system, move |ctx, message| {
    let Ok(envelope) = message.downcast::<MessageEnvelope<U>>() else {
      return Err(ActorFailure::from_message("unexpected message type delivered to typed handler"));
    };
    match envelope {
      | MessageEnvelope::User(user) => {
        let (message, metadata) = user.into_parts::<MailboxConcurrencyOf<AR>>();
        let metadata = metadata.unwrap_or_default();
        let mut typed_ctx = ActorContext::with_metadata(ctx, metadata);
        adapter.handle_user(&mut typed_ctx, message)
      },
      | MessageEnvelope::System(message) => {
        let mut typed_ctx = ActorContext::new(ctx);
        adapter.handle_system(&mut typed_ctx, message)
      },
    }
  })
}
