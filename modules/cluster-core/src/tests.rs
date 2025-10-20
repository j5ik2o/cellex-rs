use std::{
  any::Any,
  borrow::Cow,
  sync::{Arc, Mutex, MutexGuard},
};

use cellex_actor_core_rs::api::{
  actor::{
    actor_failure::{ActorFailure, BehaviorFailure},
    ActorId, ActorPath,
  },
  failure_event_stream::FailureEventStream,
  supervision::{
    escalation::FailureEventListener,
    failure::{FailureEvent, FailureInfo},
  },
};
use cellex_actor_std_rs::FailureEventHub;
use cellex_remote_core_rs::RemoteFailureNotifier;

use super::ClusterFailureBridge;

type TestResult<T = ()> = Result<T, String>;

fn lock<'a, T>(mutex: &'a Mutex<T>) -> Result<MutexGuard<'a, T>, String> {
  mutex.lock().map_err(|err| format!("mutex poisoned: {:?}", err))
}

#[test]
fn cluster_failure_bridge_new_creates_instance() {
  let hub = FailureEventHub::new();
  let remote_notifier = RemoteFailureNotifier::new(hub.clone());
  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let _listener = bridge.register();
}

#[test]
fn cluster_failure_bridge_register_returns_listener() {
  let hub = FailureEventHub::new();
  let remote_notifier = RemoteFailureNotifier::new(hub.clone());
  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let _listener = bridge.register();
}

#[test]
fn cluster_failure_bridge_notifier_returns_reference() {
  let hub = FailureEventHub::new();
  let remote_notifier = RemoteFailureNotifier::new(hub.clone());
  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let _notifier_ref = bridge.notifier();
}

#[test]
fn cluster_failure_bridge_fan_out_dispatches_root_escalation() -> TestResult {
  let hub = FailureEventHub::new();

  let hub_events = Arc::new(Mutex::new(Vec::new()));
  let hub_events_clone = Arc::clone(&hub_events);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    hub_events_clone.lock().unwrap_or_else(|err| err.into_inner()).push(event);
  }));

  let remote_hub = FailureEventHub::new();
  let mut remote_notifier = RemoteFailureNotifier::new(remote_hub);

  let handler_called = Arc::new(Mutex::new(false));
  let handler_called_clone = Arc::clone(&handler_called);

  let handler = FailureEventListener::new(move |event: FailureEvent| {
    if matches!(event, FailureEvent::RootEscalated(_)) {
      *handler_called_clone.lock().unwrap_or_else(|err| err.into_inner()) = true;
    }
  });
  remote_notifier.set_handler(handler);

  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  let event = FailureEvent::RootEscalated(info.clone());

  bridge.fan_out(event);

  assert!(*lock(&handler_called)?);

  let events = lock(&hub_events)?;
  assert_eq!(events.len(), 1);

  let FailureEvent::RootEscalated(received_info) = &events[0];

  assert_eq!(received_info.actor, info.actor);
  assert_eq!(received_info.description(), info.description());
  Ok(())
}

#[test]
fn cluster_failure_bridge_fan_out_handles_hub_listener_call() -> TestResult {
  let hub = FailureEventHub::new();

  let hub_events = Arc::new(Mutex::new(Vec::new()));
  let hub_events_clone = Arc::clone(&hub_events);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    hub_events_clone.lock().unwrap_or_else(|err| err.into_inner()).push(event);
  }));

  let remote_hub = FailureEventHub::new();
  let remote_notifier = RemoteFailureNotifier::new(remote_hub);

  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let info = FailureInfo::new(ActorId(2), ActorPath::new(), ActorFailure::from_message("another error"));
  let event = FailureEvent::RootEscalated(info);

  bridge.fan_out(event);

  let events = lock(&hub_events)?;
  assert_eq!(events.len(), 1);
  Ok(())
}

#[derive(Debug)]
struct ClusterBehaviorFailure(&'static str);

impl BehaviorFailure for ClusterBehaviorFailure {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn description(&self) -> Cow<'_, str> {
    Cow::Borrowed(self.0)
  }
}

#[test]
fn cluster_failure_bridge_preserves_behavior_failure() -> TestResult {
  let hub = FailureEventHub::new();

  let captured_local: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
  let captured_local_clone = Arc::clone(&captured_local);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    if let Some(custom) = info.behavior_failure().as_any().downcast_ref::<ClusterBehaviorFailure>() {
      captured_local_clone.lock().unwrap_or_else(|err| err.into_inner()).replace(custom.0.to_owned());
    }
  }));

  let remote_hub = FailureEventHub::new();
  let mut remote_notifier = RemoteFailureNotifier::new(remote_hub);

  let captured_remote: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
  let captured_remote_clone = Arc::clone(&captured_remote);
  remote_notifier.set_handler(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    if let Some(custom) = info.behavior_failure().as_any().downcast_ref::<ClusterBehaviorFailure>() {
      captured_remote_clone.lock().unwrap_or_else(|err| err.into_inner()).replace(custom.0.to_owned());
    }
  }));

  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let failure = ActorFailure::new(ClusterBehaviorFailure("cluster boom"));
  let info = FailureInfo::new(ActorId(5), ActorPath::new(), failure);
  bridge.fan_out(FailureEvent::RootEscalated(info));

  assert_eq!(lock(&captured_local)?.as_deref(), Some("cluster boom"));
  assert_eq!(lock(&captured_remote)?.as_deref(), Some("cluster boom"));
  Ok(())
}
