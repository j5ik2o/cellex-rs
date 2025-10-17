use super::*;
use cellex_actor_core_rs::api::actor::actor_failure::ActorFailure;
use cellex_actor_core_rs::api::actor::{ActorId, ActorPath};
use cellex_actor_core_rs::api::supervision::escalation::FailureEventListener;
use cellex_actor_core_rs::api::supervision::failure::{FailureEvent, FailureInfo, FailureMetadata};
use std::sync::Arc as StdArc;
use std::sync::Mutex as StdMutex;

#[test]
fn hub_forwards_events_to_subscribers() {
  let hub = FailureEventHub::new();
  let storage: StdArc<StdMutex<Vec<FailureEvent>>> = StdArc::new(StdMutex::new(Vec::new()));
  let storage_clone = storage.clone();

  let _sub = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    storage_clone.lock().unwrap().push(event);
  }));

  let listener = hub.listener();
  let event = FailureEvent::RootEscalated(FailureInfo::new_with_metadata(
    ActorId(1),
    ActorPath::new(),
    ActorFailure::from_message("boom"),
    FailureMetadata::default(),
  ));
  listener(event.clone());

  let events = storage.lock().unwrap();
  assert_eq!(events.len(), 1);
  match &events[0] {
    FailureEvent::RootEscalated(info) => assert_eq!(info.description().as_ref(), "boom"),
  }
}

#[test]
fn subscription_drop_removes_listener() {
  let hub = FailureEventHub::new();
  assert_eq!(hub.listener_count(), 0);
  let subscription = hub.subscribe(FailureEventListener::new(|_| {}));
  assert_eq!(hub.listener_count(), 1);
  drop(subscription);
  assert_eq!(hub.listener_count(), 0);
}
