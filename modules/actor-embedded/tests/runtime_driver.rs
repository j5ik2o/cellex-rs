#![allow(clippy::disallowed_types)]
extern crate alloc;
extern crate std;

use alloc::{rc::Rc, vec::Vec};
use core::cell::RefCell;
use std::sync::Arc;

use cellex_actor_core_rs::api::{
  actor::{actor_failure::ActorFailure, ActorId, ActorPath, Props},
  actor_runtime::GenericActorRuntime,
  actor_system::ActorSystem,
};
use cellex_actor_core_rs::api::failure::{FailureEvent, FailureInfo, FailureMetadata};
use cellex_actor_core_rs::api::failure::failure_event_stream::{FailureEventListener, FailureEventStream};
use cellex_actor_embedded_rs::{runtime_driver::EmbeddedFailureEventHub, LocalMailboxRuntime};

#[test]
fn embedded_actor_runtime_dispatches_message() {
  let hub = EmbeddedFailureEventHub::new();
  let mut system: ActorSystem<u32, _> = ActorSystem::new_with_actor_runtime_and_event_stream(
    GenericActorRuntime::new(LocalMailboxRuntime::default()),
    &hub,
  );

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let props = Props::new(move |_, msg: u32| {
    log_clone.borrow_mut().push(msg);
    Ok(())
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");

  actor_ref.tell(11).expect("tell message");
  system.run_until_idle().expect("run until idle");

  assert_eq!(log.borrow().as_slice(), &[11]);
}

#[test]
fn embedded_failure_event_hub_broadcasts() {
  let hub = EmbeddedFailureEventHub::new();

  let received = Arc::new(std::sync::Mutex::new(Vec::<FailureEvent>::new()));
  let received_clone = received.clone();

  let _subscription = hub.subscribe(FailureEventListener::new(move |event| {
    received_clone.lock().unwrap().push(event);
  }));

  let listener = hub.listener();
  let info = FailureInfo::new_with_metadata(
    ActorId(1),
    ActorPath::new(),
    ActorFailure::from_message("boom"),
    FailureMetadata::default(),
  );

  listener(FailureEvent::RootEscalated(info.clone()));

  assert_eq!(received.lock().unwrap().len(), 1);
  let guard = received.lock().unwrap();
  let FailureEvent::RootEscalated(recorded) = &guard[0];
  assert_eq!(recorded.actor, info.actor);
}
