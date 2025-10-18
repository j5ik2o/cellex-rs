use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use crate::{
  api::{
    actor::{ActorHandlerFn, ActorId, ActorPath, ChildNaming},
    actor_system::map_system::MapSystemShared,
    extensions::Extensions,
    mailbox::{MailboxFactory, PriorityEnvelope},
    messaging::DynMessage,
    process::pid::Pid,
    supervision::supervisor::Supervisor,
  },
  internal::mailbox::PriorityMailboxSpawnerHandle,
};

/// Information required when spawning child actors.
pub(crate) struct ChildSpawnSpec<MF>
where
  MF: MailboxFactory + Clone, {
  /// Mailbox instance assigned to the child actor.
  pub mailbox:         MF::Mailbox<PriorityEnvelope<DynMessage>>,
  /// Producer handle used to send messages to the child actor.
  pub sender:          MF::Producer<PriorityEnvelope<DynMessage>>,
  /// Supervisor that governs the child actor lifecycle.
  pub supervisor:      Box<dyn Supervisor<DynMessage>>,
  /// Message handler executed by the child actor.
  pub handler:         Box<ActorHandlerFn<DynMessage, MF>>,
  /// Mailbox spawner shared with the child.
  pub mailbox_spawner: PriorityMailboxSpawnerHandle<DynMessage, MF>,
  /// List of actor IDs watching the child.
  pub watchers:        Vec<ActorId>,
  /// Mapping function from system messages to the actor message type.
  pub map_system:      MapSystemShared<DynMessage>,
  /// Parent actor path for the spawned child.
  pub parent_path:     ActorPath,
  /// Shared extensions available to the child actor.
  pub extensions:      Extensions,
  /// Naming strategy applied when instantiating the child actor.
  pub child_naming:    ChildNaming,
  /// Slot used to supply the assigned PID back to API-level references.
  pub pid_slot:        ArcShared<RwLock<Option<Pid>>>,
}
