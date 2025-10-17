use crate::api::identity::ActorId;
use crate::api::identity::ActorPath;
use crate::api::mailbox::MailboxFactory;
use crate::internal::actor::InternalActorRef;
use crate::shared::map_system::MapSystemShared;
use alloc::string::String;
use cellex_utils_core_rs::Element;

#[allow(dead_code)]
pub(crate) struct ChildRecord<M, R>
where
  M: Element,
  R: MailboxFactory, {
  pub(super) control_ref: InternalActorRef<M, R>,
  pub(super) map_system: MapSystemShared<M>,
  pub(super) watcher: Option<ActorId>,
  pub(super) path: ActorPath,
  pub(super) name: Option<String>,
}
