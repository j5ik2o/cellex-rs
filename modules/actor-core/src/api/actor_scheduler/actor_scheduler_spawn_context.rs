use alloc::boxed::Box;

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ActorHandlerFn, ChildNaming},
    mailbox::{MailboxFactory, MailboxOptions},
    process::{pid::Pid, process_registry::ProcessRegistry},
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MapSystemShared},
  },
};

type SchedulerProcessRegistry<MF> =
  ArcShared<ProcessRegistry<PriorityActorRef<AnyMessage, MF>, ArcShared<PriorityEnvelope<AnyMessage>>>>;

/// Parameters supplied to schedulers when spawning a new actor.
pub struct ActorSchedulerSpawnContext<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  /// use cellex_actor_core_rs::api::mailbox::MailboxRuntime; used to create queue-backed actor
  /// mailboxes.
  pub mailbox_factory:        MF,
  /// Shared clone of the use cellex_actor_core_rs::api::mailbox::MailboxRuntime; for components
  /// that require `ArcShared` access.
  pub mailbox_factory_shared: ArcShared<MF>,
  /// Mapping utilities used to translate system messages for the target actor type.
  pub map_system:             MapSystemShared<AnyMessage>,
  /// Mailbox configuration parameters applied during actor creation.
  pub mailbox_options:        MailboxOptions,
  /// Handler invoked to execute the actor's behavior.
  pub handler:                Box<ActorHandlerFn<AnyMessage, MF>>,
  /// Naming strategy to apply when registering the child actor.
  pub child_naming:           ChildNaming,
  /// Process registry used to register and resolve actor PIDs.
  pub process_registry:       SchedulerProcessRegistry<MF>,
  /// Slot where the assigned PID will be recorded.
  pub actor_pid_slot:         ArcShared<RwLock<Option<Pid>>>,
}
