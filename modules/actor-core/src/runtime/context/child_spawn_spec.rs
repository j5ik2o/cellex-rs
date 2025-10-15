use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::runtime::mailbox::PriorityMailboxSpawnerHandle;
use crate::ActorId;
use crate::ActorPath;
use crate::ChildNaming;
use crate::Extensions;
use crate::Supervisor;
use crate::{MailboxRuntime, PriorityEnvelope};
use cellex_utils_core_rs::Element;

use super::ActorHandlerFn;
use crate::MapSystemShared;

/// Information required when spawning child actors.
pub struct ChildSpawnSpec<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone, {
  /// Mailbox instance assigned to the child actor.
  pub mailbox: R::Mailbox<PriorityEnvelope<M>>,
  /// Producer handle used to send messages to the child actor.
  pub sender: R::Producer<PriorityEnvelope<M>>,
  /// Supervisor that governs the child actor lifecycle.
  pub supervisor: Box<dyn Supervisor<M>>,
  /// Message handler executed by the child actor.
  pub handler: Box<ActorHandlerFn<M, R>>,
  /// Mailbox spawner shared with the child.
  pub mailbox_spawner: PriorityMailboxSpawnerHandle<M, R>,
  /// List of actor IDs watching the child.
  pub watchers: Vec<ActorId>,
  /// Mapping function from system messages to the actor message type.
  pub map_system: MapSystemShared<M>,
  /// Parent actor path for the spawned child.
  pub parent_path: ActorPath,
  /// Shared extensions available to the child actor.
  pub extensions: Extensions,
  /// Naming strategy applied when instantiating the child actor.
  pub child_naming: ChildNaming,
}
