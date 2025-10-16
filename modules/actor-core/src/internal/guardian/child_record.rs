use crate::internal::context::InternalActorRef;
use crate::ActorId;
use crate::ActorPath;
use crate::MailboxRuntime;
use crate::MapSystemShared;
use alloc::string::String;
use cellex_utils_core_rs::Element;

#[allow(dead_code)]
pub(crate) struct ChildRecord<M, R>
where
  M: Element,
  R: MailboxRuntime, {
  pub(super) control_ref: InternalActorRef<M, R>,
  pub(super) map_system: MapSystemShared<M>,
  pub(super) watcher: Option<ActorId>,
  pub(super) path: ActorPath,
  pub(super) name: Option<String>,
}
