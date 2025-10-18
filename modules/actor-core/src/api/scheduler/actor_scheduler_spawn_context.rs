use alloc::boxed::Box;

use cellex_utils_core_rs::{sync::ArcShared, Element};

use crate::{
  api::{
    actor::ChildNaming,
    actor_system::map_system::MapSystemShared,
    mailbox::{MailboxFactory, MailboxOptions, PriorityEnvelope},
  },
  internal::context::ActorHandlerFn,
};

/// Parameters supplied to schedulers when spawning a new actor.
pub struct ActorSchedulerSpawnContext<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  /// use cellex_actor_core_rs::api::mailbox::MailboxRuntime; used to create queue-backed actor
  /// mailboxes.
  pub mailbox_factory:        MF,
  /// Shared clone of the use cellex_actor_core_rs::api::mailbox::MailboxRuntime; for components
  /// that require `ArcShared` access.
  pub mailbox_factory_shared: ArcShared<MF>,
  /// Mapping utilities used to translate system messages for the target actor type.
  pub map_system:             MapSystemShared<M>,
  /// Mailbox configuration parameters applied during actor creation.
  pub mailbox_options:        MailboxOptions,
  /// Handler invoked to execute the actor's behavior.
  pub handler:                Box<ActorHandlerFn<M, MF>>,
  /// Naming strategy to apply when registering the child actor.
  pub child_naming:           ChildNaming,
}
