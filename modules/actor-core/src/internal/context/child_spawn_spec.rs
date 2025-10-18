use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::{sync::ArcShared, Element};
use spin::RwLock;

use super::ActorHandlerFn;
use crate::{
  api::{
    actor::{ActorId, ActorPath, ChildNaming},
    actor_system::map_system::MapSystemShared,
    extensions::Extensions,
    mailbox::{MailboxFactory, PriorityEnvelope},
    process::pid::Pid,
    supervision::supervisor::Supervisor,
  },
  internal::mailbox::PriorityMailboxSpawnerHandle,
};

/// Information required when spawning child actors.
pub struct ChildSpawnSpec<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone, {
  /// Mailbox instance assigned to the child actor.
  pub mailbox:         MF::Mailbox<PriorityEnvelope<M>>,
  /// Producer handle used to send messages to the child actor.
  pub sender:          MF::Producer<PriorityEnvelope<M>>,
  /// Supervisor that governs the child actor lifecycle.
  pub supervisor:      Box<dyn Supervisor<M>>,
  /// Message handler executed by the child actor.
  pub handler:         Box<ActorHandlerFn<M, MF>>,
  /// Mailbox spawner shared with the child.
  pub mailbox_spawner: PriorityMailboxSpawnerHandle<M, MF>,
  /// List of actor IDs watching the child.
  pub watchers:        Vec<ActorId>,
  /// Mapping function from system messages to the actor message type.
  pub map_system:      MapSystemShared<M>,
  /// Parent actor path for the spawned child.
  pub parent_path:     ActorPath,
  /// Shared extensions available to the child actor.
  pub extensions:      Extensions,
  /// Naming strategy applied when instantiating the child actor.
  pub child_naming:    ChildNaming,
  /// Slot used to supply the assigned PID back to API-level references.
  pub pid_slot:        ArcShared<RwLock<Option<Pid>>>,
}
