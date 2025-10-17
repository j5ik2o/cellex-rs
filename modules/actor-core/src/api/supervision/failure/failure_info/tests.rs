use super::*;
use crate::api::actor::actor_failure::ActorFailure;
use crate::api::actor::actor_failure::DefaultBehaviorFailure;

#[test]
fn failure_info_escalate_preserves_failure() {
  let child = ActorId(2);
  let path = ActorPath::new().push_child(ActorId(0)).push_child(child);
  let original = ActorFailure::from_message("boom");
  let info = FailureInfo::from_failure(child, path.clone(), original.clone());

  assert!(info.description().contains("boom"));
  assert!(info.behavior_failure().as_any().is::<DefaultBehaviorFailure>());

  let parent = info.escalate_to_parent().expect("parent failure");
  assert_eq!(parent.path.segments().last().copied(), path.parent().unwrap().last());
  assert_eq!(parent.description(), original.description());
}
