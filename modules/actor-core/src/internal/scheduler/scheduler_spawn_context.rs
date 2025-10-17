use alloc::boxed::Box;

use crate::api::actor_system::map_system::MapSystemShared;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::MailboxOptions;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::context::ActorHandlerFn;
use crate::internal::scheduler::child_naming::ChildNaming;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

/// Parameters supplied to schedulers when spawning a new actor.
pub struct SchedulerSpawnContext<M, R>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// use cellex_actor_core_rs::api::mailbox::MailboxRuntime; used to create queue-backed actor mailboxes.
  pub mailbox_factory: R,
  /// Shared clone of the use cellex_actor_core_rs::api::mailbox::MailboxRuntime; for components that require `ArcShared` access.
  pub mailbox_factory_shared: ArcShared<R>,
  /// Mapping utilities used to translate system messages for the target actor type.
  pub map_system: MapSystemShared<M>,
  /// Mailbox configuration parameters applied during actor creation.
  pub mailbox_options: MailboxOptions,
  /// Handler invoked to execute the actor's behavior.
  pub handler: Box<ActorHandlerFn<M, R>>,
  /// Naming strategy to apply when registering the child actor.
  pub child_naming: ChildNaming,
}
