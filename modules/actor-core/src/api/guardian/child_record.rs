use alloc::string::String;

use crate::api::{
  actor::{actor_ref::PriorityActorRef, ActorId, ActorPath},
  actor_system::map_system::MapSystemShared,
  mailbox::MailboxFactory,
  messaging::DynMessage,
};

#[allow(dead_code)]
pub(crate) struct ChildRecord<MF>
where
  MF: MailboxFactory, {
  pub(crate) control_ref: PriorityActorRef<DynMessage, MF>,
  pub(crate) map_system:  MapSystemShared<DynMessage>,
  pub(crate) watcher:     Option<ActorId>,
  pub(crate) path:        ActorPath,
  pub(crate) name:        Option<String>,
}
