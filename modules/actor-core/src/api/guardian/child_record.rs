use alloc::string::String;

use crate::{
  api::actor::{actor_ref::PriorityActorRef, ActorId, ActorPath},
  shared::{
    mailbox::MailboxFactory,
    messaging::{AnyMessage, MapSystemShared},
  },
};

#[allow(dead_code)]
pub(crate) struct ChildRecord<MF>
where
  MF: MailboxFactory, {
  pub(crate) control_ref: PriorityActorRef<AnyMessage, MF>,
  pub(crate) map_system:  MapSystemShared<AnyMessage>,
  pub(crate) watcher:     Option<ActorId>,
  pub(crate) path:        ActorPath,
  pub(crate) name:        Option<String>,
}
