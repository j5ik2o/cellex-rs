use std::sync::{Arc, Mutex};

use cellex_actor_core_rs::api::{
  actor::{actor_failure::ActorFailure, ActorId, ActorPath},
  failure_event_stream::FailureEventStream,
  mailbox::{PriorityChannel, PriorityEnvelope, SystemMessage},
  supervision::{
    escalation::FailureEventListener,
    failure::{FailureEvent, FailureInfo},
  },
};
use cellex_actor_std_rs::FailureEventHub;

use super::{placeholder_metadata, RemoteFailureNotifier};
use crate::remote_envelope::RemoteEnvelope;

#[test]
fn remote_failure_notifier_new_creates_instance() {
  let hub = FailureEventHub::new();
  let notifier = RemoteFailureNotifier::new(hub);

  assert!(notifier.handler().is_none());
}

#[test]
fn remote_failure_notifier_listener_returns_hub_listener() {
  let hub = FailureEventHub::new();
  let notifier = RemoteFailureNotifier::new(hub);

  let _listener = notifier.listener();
}

#[test]
fn remote_failure_notifier_hub_returns_reference() {
  let hub = FailureEventHub::new();
  let notifier = RemoteFailureNotifier::new(hub);

  let _hub_ref = notifier.hub();
}

#[test]
fn remote_failure_notifier_set_handler_stores_handler() {
  let hub = FailureEventHub::new();
  let mut notifier = RemoteFailureNotifier::new(hub);

  assert!(notifier.handler().is_none());

  let handler = FailureEventListener::new(|_event: FailureEvent| {});
  notifier.set_handler(handler);

  assert!(notifier.handler().is_some());
}

#[test]
fn remote_failure_notifier_dispatch_calls_handler() {
  let hub = FailureEventHub::new();
  let mut notifier = RemoteFailureNotifier::new(hub);

  let called = Arc::new(Mutex::new(false));
  let called_clone = Arc::clone(&called);

  let handler = FailureEventListener::new(move |event: FailureEvent| {
    if matches!(event, FailureEvent::RootEscalated(_)) {
      *called_clone.lock().unwrap() = true;
    }
  });
  notifier.set_handler(handler);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  notifier.dispatch(info);

  assert!(*called.lock().unwrap());
}

#[test]
fn remote_failure_notifier_dispatch_without_handler_does_nothing() {
  let hub = FailureEventHub::new();
  let notifier = RemoteFailureNotifier::new(hub);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  notifier.dispatch(info);
}

#[test]
fn remote_failure_notifier_emit_calls_hub_and_handler() {
  let hub = FailureEventHub::new();

  let hub_events = Arc::new(Mutex::new(Vec::new()));
  let hub_events_clone = Arc::clone(&hub_events);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    hub_events_clone.lock().unwrap().push(event);
  }));

  let mut notifier = RemoteFailureNotifier::new(hub);

  let handler_called = Arc::new(Mutex::new(false));
  let handler_called_clone = Arc::clone(&handler_called);

  let handler = FailureEventListener::new(move |event: FailureEvent| {
    if matches!(event, FailureEvent::RootEscalated(_)) {
      *handler_called_clone.lock().unwrap() = true;
    }
  });
  notifier.set_handler(handler);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  notifier.emit(info.clone());

  assert!(*handler_called.lock().unwrap());

  let events = hub_events.lock().unwrap();
  assert_eq!(events.len(), 1);

  let FailureEvent::RootEscalated(received_info) = &events[0];
  assert_eq!(received_info.actor, info.actor);
  assert_eq!(received_info.description(), info.description());
}

#[test]
fn placeholder_metadata_creates_metadata_with_endpoint() {
  let endpoint = "localhost:8080";
  let metadata = placeholder_metadata(endpoint);

  assert_eq!(metadata.endpoint, Some(endpoint.to_string()));
  assert!(metadata.component.is_none());
  assert!(metadata.transport.is_none());
  assert!(metadata.tags.is_empty());
}

#[test]
fn remote_envelope_roundtrip_preserves_control_channel() {
  let system_msg = SystemMessage::Stop;
  let expected_priority = system_msg.priority();
  let priority_envelope = PriorityEnvelope::from_system(system_msg.clone());

  let remote_envelope: RemoteEnvelope<SystemMessage> = RemoteEnvelope::from(priority_envelope);

  assert_eq!(remote_envelope.priority(), expected_priority);
  assert_eq!(remote_envelope.channel(), PriorityChannel::Control);

  let restored: PriorityEnvelope<SystemMessage> = remote_envelope.into();
  assert_eq!(restored.channel(), PriorityChannel::Control);
  assert_eq!(restored.priority(), expected_priority);
  let (restored_message, priority, channel) = restored.into_parts_with_channel();
  assert_eq!(restored_message, system_msg);
  assert_eq!(priority, expected_priority);
  assert_eq!(channel, PriorityChannel::Control);
}

#[test]
fn remote_envelope_roundtrip_preserves_user_priority() {
  let priority_envelope = PriorityEnvelope::control("ping".to_string(), 7);

  let remote_envelope: RemoteEnvelope<String> = RemoteEnvelope::from(priority_envelope);

  assert_eq!(remote_envelope.priority(), 7);
  assert_eq!(remote_envelope.channel(), PriorityChannel::Control);
  assert_eq!(remote_envelope.message(), "ping");

  let restored: PriorityEnvelope<String> = remote_envelope.into();
  assert_eq!(restored.channel(), PriorityChannel::Control);
  assert_eq!(restored.priority(), 7);
  let (message, priority, channel) = restored.into_parts_with_channel();
  assert_eq!(message, "ping");
  assert_eq!(priority, 7);
  assert_eq!(channel, PriorityChannel::Control);
}
