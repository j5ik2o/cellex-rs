use super::Context;
use crate::api::actor::{ActorRef, Props};
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::PriorityEnvelope;
use crate::{DynMessage, MailboxRuntime, MetadataStorageMode};
use alloc::boxed::Box;
use cellex_utils_core_rs::Element;

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Spawns a child actor and returns an `ActorRef`.
  pub fn spawn_child<V>(&mut self, props: Props<V, R>) -> ActorRef<V, R>
  where
    V: Element, {
    let (internal_props, supervisor_cfg) = props.into_parts();
    let actor_ref = self
      .inner
      .spawn_child_from_props(Box::new(supervisor_cfg.as_supervisor()), internal_props);
    ActorRef::new(actor_ref)
  }
}
