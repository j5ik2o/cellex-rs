use alloc::string::String;

use cellex_utils_core_rs::Element;

use crate::api::{
  actor::{actor_ref::PriorityActorRef, ActorId, ActorPath},
  actor_system::map_system::MapSystemShared,
  mailbox::MailboxFactory,
};

#[allow(dead_code)]
pub(crate) struct ChildRecord<M, MF>
where
  M: Element,
  MF: MailboxFactory, {
  pub(super) control_ref: PriorityActorRef<M, MF>,
  pub(super) map_system:  MapSystemShared<M>,
  pub(super) watcher:     Option<ActorId>,
  pub(super) path:        ActorPath,
  pub(super) name:        Option<String>,
}
