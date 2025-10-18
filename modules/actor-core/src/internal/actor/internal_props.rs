use alloc::boxed::Box;

use cellex_utils_core_rs::Element;

use crate::api::{
  actor::{actor_failure::ActorFailure, context::Context, ActorContext, ActorHandlerFn},
  actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
  actor_system::map_system::MapSystemShared,
  mailbox::{MailboxFactory, MailboxOptions, PriorityEnvelope},
  messaging::{DynMessage, MessageEnvelope, MetadataStorageMode},
};

pub(crate) struct InternalProps<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone, {
  pub options:    MailboxOptions,
  pub map_system: MapSystemShared<DynMessage>,
  pub handler:    Box<ActorHandlerFn<DynMessage, MF>>,
}

impl<MF> InternalProps<MF>
where
  MF: MailboxFactory + Clone,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone,
{
  pub fn new(
    options: MailboxOptions,
    map_system: MapSystemShared<DynMessage>,
    handler: impl for<'ctx> FnMut(&mut ActorContext<'ctx, MF>, DynMessage) -> Result<(), ActorFailure> + 'static,
  ) -> Self {
    Self { options, map_system, handler: Box::new(handler) }
  }
}

pub(crate) fn internal_props_from_adapter<U, AR>(
  options: MailboxOptions,
  map_system: MapSystemShared<DynMessage>,
  mut adapter: crate::api::actor::behavior::ActorAdapter<U, AR>,
) -> InternalProps<MailboxOf<AR>>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  InternalProps::new(options, map_system, move |ctx, message| {
    let Ok(envelope) = message.downcast::<MessageEnvelope<U>>() else {
      panic!("unexpected message type delivered to typed handler");
    };
    match envelope {
      | MessageEnvelope::User(user) => {
        let (message, metadata) = user.into_parts::<MailboxConcurrencyOf<AR>>();
        let metadata = metadata.unwrap_or_default();
        let mut typed_ctx = Context::with_metadata(ctx, metadata);
        adapter.handle_user(&mut typed_ctx, message)
      },
      | MessageEnvelope::System(message) => {
        let mut typed_ctx = Context::new(ctx);
        adapter.handle_system(&mut typed_ctx, message)
      },
    }
  })
}
