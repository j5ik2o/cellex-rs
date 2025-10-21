use std::sync::{Arc as StdArc, Mutex as StdMutex};

use cellex_actor_core_rs::api::{
  actor::{actor_failure::ActorFailure, ActorId, ActorPath},
  failure::{
    failure_event_stream::{FailureEventListener, FailureEventStream},
    FailureEvent, FailureInfo, FailureMetadata,
  },
};

use super::failure_event_hub_impl::FailureEventHub;

impl FailureEventHub {
  fn listener_count(&self) -> usize {
    self.inner.lock_listeners().len()
  }
}

#[test]
fn hub_forwards_events_to_subscribers() {
  let hub = FailureEventHub::new();
  let storage: StdArc<StdMutex<Vec<FailureEvent>>> = StdArc::new(StdMutex::new(Vec::new()));
  let storage_clone = storage.clone();

  let _sub = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    storage_clone.lock().unwrap_or_else(|err| err.into_inner()).push(event);
  }));

  let listener = hub.listener();
  let event = FailureEvent::RootEscalated(FailureInfo::new_with_metadata(
    ActorId(1),
    ActorPath::new(),
    ActorFailure::from_message("boom"),
    FailureMetadata::default(),
  ));
  listener(event);

  let events = storage.lock().unwrap_or_else(|err| err.into_inner());
  assert_eq!(events.len(), 1);
  match &events[0] {
    | FailureEvent::RootEscalated(info) => assert_eq!(info.description().as_ref(), "boom"),
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
