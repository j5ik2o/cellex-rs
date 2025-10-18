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
  pub(crate) control_ref: PriorityActorRef<M, MF>,
  pub(crate) map_system:  MapSystemShared<M>,
  pub(crate) watcher:     Option<ActorId>,
  pub(crate) path:        ActorPath,
  pub(crate) name:        Option<String>,
}
