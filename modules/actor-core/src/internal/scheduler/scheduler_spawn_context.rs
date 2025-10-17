use alloc::boxed::Box;

use crate::api::mailbox::MailboxOptions;
use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::context::ActorHandlerFn;
use crate::internal::scheduler::child_naming::ChildNaming;
use crate::shared::map_system::MapSystemShared;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

/// Parameters supplied to schedulers when spawning a new actor.
pub struct SchedulerSpawnContext<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Mailbox runtime used to create queue-backed actor mailboxes.
  pub mailbox_runtime: R,
  /// Shared clone of the mailbox runtime for components that require `ArcShared` access.
  pub mailbox_runtime_shared: ArcShared<R>,
  /// Mapping utilities used to translate system messages for the target actor type.
  pub map_system: MapSystemShared<M>,
  /// Mailbox configuration parameters applied during actor creation.
  pub mailbox_options: MailboxOptions,
  /// Handler invoked to execute the actor's behavior.
  pub handler: Box<ActorHandlerFn<M, R>>,
  /// Naming strategy to apply when registering the child actor.
  pub child_naming: ChildNaming,
}
