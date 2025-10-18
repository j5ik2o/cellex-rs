use crate::api::actor::actor_failure::ActorFailure;
use crate::api::actor_system::map_system::MapSystemShared;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::MailboxOptions;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::supervision::supervisor::Supervisor;
use crate::internal::context::{ActorContext, ActorHandlerFn};
use alloc::boxed::Box;
use cellex_utils_core_rs::Element;

pub(crate) struct InternalProps<M, MF>
where
  M: Element + 'static,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  pub options: MailboxOptions,
  pub map_system: MapSystemShared<M>,
  pub handler: Box<ActorHandlerFn<M, MF>>,
}

impl<M, MF> InternalProps<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
{
  pub fn new(
    options: MailboxOptions,
    map_system: MapSystemShared<M>,
    handler: impl for<'ctx> FnMut(&mut ActorContext<'ctx, M, MF, dyn Supervisor<M>>, M) -> Result<(), ActorFailure>
      + 'static,
  ) -> Self {
    Self {
      options,
      map_system,
      handler: Box::new(handler),
    }
  }
}
