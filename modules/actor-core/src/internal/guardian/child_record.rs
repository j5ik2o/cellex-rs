use crate::api::actor::actor_ref::PriorityActorRef;
use crate::api::actor::ActorId;
use crate::api::actor::ActorPath;
use crate::api::actor_system::map_system::MapSystemShared;
use crate::api::mailbox::MailboxFactory;
use alloc::string::String;
use cellex_utils_core_rs::Element;

#[allow(dead_code)]
pub(crate) struct ChildRecord<M, R>
where
  M: Element,
  R: MailboxFactory, {
  pub(super) control_ref: PriorityActorRef<M, R>,
  pub(super) map_system: MapSystemShared<M>,
  pub(super) watcher: Option<ActorId>,
  pub(super) path: ActorPath,
  pub(super) name: Option<String>,
}
