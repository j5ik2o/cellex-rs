use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use crate::{
  api::{
    actor::{ActorHandlerFn, ActorId, ActorPath, ChildNaming},
    extensions::Extensions,
    process::pid::Pid,
    supervision::supervisor::Supervisor,
  },
  internal::mailbox::PriorityMailboxSpawnerHandle,
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::{AnyMessage, MapSystemShared},
  },
};

/// Information required when spawning child actors.
pub(crate) struct ChildSpawnSpec<MF>
where
  MF: MailboxFactory + Clone, {
  /// Mailbox instance assigned to the child actor.
  pub mailbox:         MF::Mailbox<PriorityEnvelope<AnyMessage>>,
  /// Producer handle used to send messages to the child actor.
  pub sender:          MF::Producer<PriorityEnvelope<AnyMessage>>,
  /// Supervisor that governs the child actor lifecycle.
  pub supervisor:      Box<dyn Supervisor<AnyMessage>>,
  /// Message handler executed by the child actor.
  pub handler:         Box<ActorHandlerFn<AnyMessage, MF>>,
  /// Mailbox spawner shared with the child.
  pub mailbox_spawner: PriorityMailboxSpawnerHandle<AnyMessage, MF>,
  /// List of actor IDs watching the child.
  pub watchers:        Vec<ActorId>,
  /// Mapping function from system messages to the actor message type.
  pub map_system:      MapSystemShared<AnyMessage>,
  /// Parent actor path for the spawned child.
  pub parent_path:     ActorPath,
  /// Shared extensions available to the child actor.
  pub extensions:      Extensions,
  /// Naming strategy applied when instantiating the child actor.
  pub child_naming:    ChildNaming,
  /// Slot used to supply the assigned PID back to API-level references.
  pub pid_slot:        ArcShared<RwLock<Option<Pid>>>,
}
