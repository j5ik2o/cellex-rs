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
  failure_event_stream::{FailureEventListener, FailureEventStream},
  failure_telemetry::{FailureSnapshot, FailureTelemetry, FailureTelemetryShared},
  mailbox::{
    messages::{PriorityChannel, PriorityEnvelope, SystemMessage},
    ThreadSafe,
  },
  messaging::MessageEnvelope,
  metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
  process::pid::{NodeId, Pid, SystemId},
  supervision::{
    escalation::{EscalationSink, RootEscalationSink},
    failure::{FailureEvent, FailureInfo},
    telemetry::TelemetryObservationConfig,
  },
  test_support::TestMailboxFactory,
};
use cellex_actor_std_rs::FailureEventHub;
use cellex_serialization_core_rs::{
  impl_type_key, InMemorySerializerRegistry, SerializationRouter, TypeBindingRegistry, TypeKey,
};
use cellex_serialization_json_rs::{shared_json_serializer, SerdeJsonSerializer, SERDE_JSON_SERIALIZER_ID};
use serde::{Deserialize, Serialize};
use serde_json::from_slice;

use super::{placeholder_metadata, RemoteFailureNotifier};
use crate::{
  codec::{
    control_remote_envelope_with_reply, envelope_from_frame, frame_from_serialized_envelope, RemoteMessageFrame,
    RemotePayloadFrame,
  },
  remote_envelope::RemoteEnvelope,
};

type TestResult<T = ()> = Result<T, String>;

fn lock<'a, T>(mutex: &'a Mutex<T>) -> Result<MutexGuard<'a, T>, String> {
  mutex.lock().map_err(|err| format!("mutex poisoned: {:?}", err))
}

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
fn remote_failure_notifier_dispatch_calls_handler() -> TestResult {
  let hub = FailureEventHub::new();
  let mut notifier = RemoteFailureNotifier::new(hub);

  let called = Arc::new(Mutex::new(false));
  let called_clone = Arc::clone(&called);

  let handler = FailureEventListener::new(move |event: FailureEvent| {
    if matches!(event, FailureEvent::RootEscalated(_)) {
      *called_clone.lock().unwrap_or_else(|err| err.into_inner()) = true;
    }
  });
  notifier.set_handler(handler);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  notifier.dispatch(info);

  assert!(*lock(&called)?);
  Ok(())
}

#[test]
fn remote_failure_notifier_dispatch_without_handler_does_nothing() {
  let hub = FailureEventHub::new();
  let notifier = RemoteFailureNotifier::new(hub);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  notifier.dispatch(info);
}

#[test]
fn remote_failure_notifier_emit_calls_hub_and_handler() -> TestResult {
  let hub = FailureEventHub::new();

  let hub_events = Arc::new(Mutex::new(Vec::new()));
  let hub_events_clone = Arc::clone(&hub_events);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    hub_events_clone.lock().unwrap_or_else(|err| err.into_inner()).push(event);
  }));

  let mut notifier = RemoteFailureNotifier::new(hub);

  let handler_called = Arc::new(Mutex::new(false));
  let handler_called_clone = Arc::clone(&handler_called);

  let handler = FailureEventListener::new(move |event: FailureEvent| {
    if matches!(event, FailureEvent::RootEscalated(_)) {
      *handler_called_clone.lock().unwrap_or_else(|err| err.into_inner()) = true;
    }
  });
  notifier.set_handler(handler);

  let info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("test error"));
  notifier.emit(info.clone());

  assert!(*lock(&handler_called)?);

  let events = lock(&hub_events)?;
  assert_eq!(events.len(), 1);

  let FailureEvent::RootEscalated(received_info) = &events[0];
  assert_eq!(received_info.actor, info.actor);
  assert_eq!(received_info.description(), info.description());
  Ok(())
}

#[derive(Debug)]
struct SampleBehaviorFailure(&'static str);

impl BehaviorFailure for SampleBehaviorFailure {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn description(&self) -> Cow<'_, str> {
    Cow::Borrowed(self.0)
  }
}

#[test]
fn remote_failure_notifier_preserves_behavior_failure() -> TestResult {
  let hub = FailureEventHub::new();
  let mut notifier = RemoteFailureNotifier::new(hub.clone());

  let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
  let captured_clone = Arc::clone(&captured);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    if let Some(failure) = info.behavior_failure().as_any().downcast_ref::<SampleBehaviorFailure>() {
      captured_clone.lock().unwrap_or_else(|err| err.into_inner()).replace(failure.0.to_owned());
    }
  }));

  let handler_called = Arc::new(Mutex::new(false));
  let handler_called_clone = Arc::clone(&handler_called);
  notifier.set_handler(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    if info.behavior_failure().as_any().is::<SampleBehaviorFailure>() {
      *handler_called_clone.lock().unwrap_or_else(|err| err.into_inner()) = true;
    }
  }));

  let failure = ActorFailure::new(SampleBehaviorFailure("remote boom"));
  let info = FailureInfo::new(ActorId(99), ActorPath::new(), failure);
  notifier.emit(info);

  assert!(*lock(&handler_called)?, "handler should receive SampleBehaviorFailure");
  let recorded = lock(&captured)?.clone().ok_or_else(|| "captured failure".to_string())?;
  assert_eq!(recorded, "remote boom");
  Ok(())
}

#[derive(Clone, Default)]
struct RecordingTelemetry {
  events: Arc<Mutex<Vec<String>>>,
}

impl RecordingTelemetry {
  fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    (Self { events: events.clone() }, events)
  }
}

impl FailureTelemetry for RecordingTelemetry {
  fn on_failure(&self, snapshot: &FailureSnapshot) {
    let mut guard = self.events.lock().unwrap_or_else(|err| err.into_inner());
    guard.push(snapshot.description().to_owned());
  }
}

#[derive(Clone, Default)]
struct RecordingMetricsSink {
  events: Arc<Mutex<Vec<MetricsEvent>>>,
}

impl RecordingMetricsSink {
  fn new() -> (Self, Arc<Mutex<Vec<MetricsEvent>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    (Self { events: events.clone() }, events)
  }
}

impl MetricsSink for RecordingMetricsSink {
  fn record(&self, event: MetricsEvent) {
    let mut guard = self.events.lock().unwrap_or_else(|err| err.into_inner());
    guard.push(event);
  }
}

#[test]
fn remote_failure_notifier_triggers_telemetry_metrics() {
  let hub = FailureEventHub::new();
  let notifier = RemoteFailureNotifier::new(hub.clone());

  let (telemetry_impl, telemetry_events) = RecordingTelemetry::new();
  let telemetry = FailureTelemetryShared::new(telemetry_impl);

  let (metrics_impl, metrics_events) = RecordingMetricsSink::new();
  let metrics = MetricsSinkShared::new(metrics_impl);

  let mut observation = TelemetryObservationConfig::new().with_metrics_sink(metrics);
  observation.set_record_timing(true);

  let mut root_sink: RootEscalationSink<TestMailboxFactory> = RootEscalationSink::new();
  root_sink.set_telemetry(telemetry);
  root_sink.set_observation_config(observation);

  let sink = Arc::new(Mutex::new(root_sink));
  let sink_clone: Arc<Mutex<RootEscalationSink<TestMailboxFactory>>> = Arc::clone(&sink);
  let _subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    let handle_result = sink_clone.lock().unwrap_or_else(|err| err.into_inner()).handle(info, false);
    assert!(handle_result.is_ok(), "root sink should handle failure: {:?}", handle_result.err());
  }));

  let failure = ActorFailure::from_message("remote telemetry failure");
  let info = FailureInfo::new(ActorId(11), ActorPath::new(), failure);
  notifier.emit(info);

  let descriptions = telemetry_events.lock().unwrap_or_else(|err| err.into_inner()).clone();
  assert_eq!(descriptions, vec!["remote telemetry failure".to_string()]);

  let recorded_metrics = metrics_events.lock().unwrap_or_else(|err| err.into_inner()).clone();
  assert!(recorded_metrics.contains(&MetricsEvent::TelemetryInvoked));
  assert!(recorded_metrics.iter().any(|event| matches!(event, MetricsEvent::TelemetryLatencyNanos(_))));
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RemoteRouterPayload {
  value: String,
}

impl_type_key!(RemoteRouterPayload, "test.RemoteRouterPayload");

#[test]
fn remote_serialization_router_roundtrip_with_fallback() -> TestResult {
  let bindings = TypeBindingRegistry::new();
  let serializers = InMemorySerializerRegistry::new();
  let router = SerializationRouter::with_fallback(bindings, serializers.clone(), Some(SERDE_JSON_SERIALIZER_ID));

  serializers.register(shared_json_serializer()).map_err(|err| format!("serializer 登録に失敗しました: {err}"))?;

  let serializer = SerdeJsonSerializer::new();
  let payload = RemoteRouterPayload { value: "hello".to_string() };

  let serialized = serializer
    .serialize_value(Some(<RemoteRouterPayload as TypeKey>::type_key()), &payload)
    .map_err(|err| format!("シリアライズに失敗しました: {err}"))?;

  let envelope = control_remote_envelope_with_reply(serialized, 4, None);
  let frame = frame_from_serialized_envelope(envelope).map_err(|err| format!("フレーム生成に失敗しました: {err:?}"))?;

  let decoded = envelope_from_frame(frame);
  let (envelope, priority, channel) = decoded.into_parts_with_channel();
  assert_eq!(priority, 4);
  assert_eq!(channel, PriorityChannel::Control);

  let MessageEnvelope::User(user) = envelope else {
    return Err("ユーザーメッセージを期待しました".to_string());
  };

  let (message, _) = user.into_parts::<ThreadSafe>();
  let type_key = message.type_name.clone().ok_or_else(|| "型キーが存在しません".to_string())?;

  let resolved = router
    .resolve_or_fallback(&type_key)
    .ok_or_else(|| "フォールバックを含めシリアライザを解決できません".to_string())?;
  assert_eq!(resolved.serializer_id(), SERDE_JSON_SERIALIZER_ID);

  let payload_bytes = resolved.deserialize(&message).map_err(|err| format!("デシリアライズに失敗しました: {err}"))?;

  let decoded: RemoteRouterPayload =
    from_slice(&payload_bytes).map_err(|err| format!("JSON 変換に失敗しました: {err}"))?;
  assert_eq!(decoded, payload);
  Ok(())
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
#[allow(clippy::unnecessary_wraps)]
fn remote_envelope_roundtrip_preserves_control_channel() -> TestResult {
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
  Ok(())
}

#[test]
#[allow(clippy::unnecessary_wraps)]
fn remote_envelope_roundtrip_preserves_user_priority() -> TestResult {
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
  Ok(())
}

#[test]
fn remote_envelope_roundtrip_preserves_user_message_envelope() -> TestResult {
  let message_envelope = MessageEnvelope::user("hello".to_string());
  let priority_envelope = PriorityEnvelope::with_channel(message_envelope, 5, PriorityChannel::Control);

  let remote_envelope: RemoteEnvelope<MessageEnvelope<String>> = RemoteEnvelope::from(priority_envelope);

  assert_eq!(remote_envelope.priority(), 5);
  assert_eq!(remote_envelope.channel(), PriorityChannel::Control);

  let restored: PriorityEnvelope<MessageEnvelope<String>> = remote_envelope.into();
  let (restored_envelope, priority, channel) = restored.into_parts_with_channel();

  assert_eq!(priority, 5);
  assert_eq!(channel, PriorityChannel::Control);

  match restored_envelope {
    | MessageEnvelope::User(user) => {
      let (message, metadata) = user.into_parts::<ThreadSafe>();
      assert_eq!(message, "hello");
      assert!(metadata.is_none());
    },
    | MessageEnvelope::System(_) => return Err("expected user envelope".to_string()),
  }
  Ok(())
}

#[test]
fn frame_roundtrip_preserves_channel_and_priority_for_system_message() -> TestResult {
  let priority = SystemMessage::Restart.priority();
  let message_envelope = MessageEnvelope::System(SystemMessage::Restart);
  let envelope = RemoteEnvelope::new(message_envelope, priority, PriorityChannel::Control);
  let frame = frame_from_serialized_envelope(envelope).map_err(|err| format!("frame encoding: {:?}", err))?;

  assert_eq!(frame.priority, priority);
  assert_eq!(frame.channel, PriorityChannel::Control);
  match &frame.payload {
    | RemotePayloadFrame::System(message) => assert!(matches!(message, SystemMessage::Restart)),
    | _ => return Err("expected system payload".to_string()),
  }

  let decoded = envelope_from_frame(frame);
  let (decoded_message, decoded_priority, decoded_channel) = decoded.into_parts_with_channel();
  assert_eq!(decoded_priority, priority);
  assert_eq!(decoded_channel, PriorityChannel::Control);
  assert!(matches!(decoded_message, MessageEnvelope::System(SystemMessage::Restart)));
  Ok(())
}

#[test]
fn frame_roundtrip_preserves_serialized_user_payload() -> TestResult {
  let serializer = SerdeJsonSerializer::new();
  let serialized = serializer
    .serialize_value(Some("String"), &"hello".to_string())
    .map_err(|err| format!("serialize payload: {:?}", err))?;

  let reply_to = Some(Pid::new(SystemId::new("sys"), ActorPath::new()).with_node(NodeId::new("remote", Some(2552))));

  let envelope = control_remote_envelope_with_reply(serialized.clone(), 9, reply_to.clone());
  let frame = frame_from_serialized_envelope(envelope).map_err(|err| format!("frame encoding: {:?}", err))?;

  assert_eq!(frame.priority, 9);
  assert_eq!(frame.channel, PriorityChannel::Control);
  assert_eq!(frame.reply_to.as_ref().map(ToString::to_string), reply_to.as_ref().map(ToString::to_string));

  let RemotePayloadFrame::User { serialized: frame_payload } = &frame.payload else {
    return Err("expected user payload".to_string());
  };
  assert_eq!(frame_payload.serializer_id, serialized.serializer_id);
  assert_eq!(frame_payload.payload, serialized.payload);

  let decoded = envelope_from_frame(frame);
  let (decoded_envelope, priority, channel) = decoded.into_parts_with_channel();
  assert_eq!(priority, 9);
  assert_eq!(channel, PriorityChannel::Control);
  match decoded_envelope {
    | MessageEnvelope::User(user) => {
      let (payload, metadata) = user.into_parts::<ThreadSafe>();
      let metadata = metadata.ok_or_else(|| "metadata expected".to_string())?;
      assert_eq!(metadata.responder_pid().map(ToString::to_string), reply_to.as_ref().map(ToString::to_string));
      assert_eq!(payload.payload, serialized.payload);
    },
    | MessageEnvelope::System(_) => return Err("expected user payload".to_string()),
  }
  Ok(())
}

#[test]
fn frame_roundtrip_preserves_reply_to_pid() -> TestResult {
  let serializer = SerdeJsonSerializer::new();
  let serialized = serializer
    .serialize_value(Some("String"), &"ping".to_string())
    .map_err(|err| format!("serialize payload: {:?}", err))?;
  let reply_to = Pid::new(SystemId::new("sys"), ActorPath::new()).with_node(NodeId::new("node", None));

  let envelope = control_remote_envelope_with_reply(serialized, 5, Some(reply_to.clone()));
  let frame = frame_from_serialized_envelope(envelope).map_err(|err| format!("frame encoding: {:?}", err))?;
  assert_eq!(frame.reply_to.as_ref().map(ToString::to_string), Some(reply_to.to_string()));

  let RemoteMessageFrame { priority, channel, payload, reply_to: reply_to_frame } = frame;
  let decoded_frame = RemoteMessageFrame::new(priority, channel, payload, reply_to_frame);
  let decoded = envelope_from_frame(decoded_frame);
  let (envelope, _, _) = decoded.into_parts_with_channel();
  match envelope {
    | MessageEnvelope::User(user) => {
      let (_, metadata) = user.into_parts::<ThreadSafe>();
      let metadata = metadata.ok_or_else(|| "metadata expected".to_string())?;
      assert_eq!(metadata.responder_pid().map(ToString::to_string), Some(reply_to.to_string()));
    },
    | MessageEnvelope::System(_) => return Err("expected user envelope".to_string()),
  }
  Ok(())
}
