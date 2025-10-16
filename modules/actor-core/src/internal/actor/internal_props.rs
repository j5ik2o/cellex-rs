use crate::api::mailbox::PriorityEnvelope;
use crate::internal::context::{ActorContext, ActorHandlerFn};
use crate::{ActorFailure, MailboxOptions, MailboxRuntime, MapSystemShared, Supervisor};
use alloc::boxed::Box;
use cellex_utils_core_rs::Element;

pub(crate) struct InternalProps<M, R>
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  pub options: MailboxOptions,
  pub map_system: MapSystemShared<M>,
  pub handler: Box<ActorHandlerFn<M, R>>,
}

impl<M, R> InternalProps<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  pub fn new(
    options: MailboxOptions,
    map_system: MapSystemShared<M>,
    handler: impl for<'ctx> FnMut(&mut ActorContext<'ctx, M, R, dyn Supervisor<M>>, M) -> Result<(), ActorFailure> + 'static,
  ) -> Self {
    Self {
      options,
      map_system,
      handler: Box::new(handler),
    }
  }
}
