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
  metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
  supervision::{
    escalation::{EscalationSink, RootEscalationSink},
    failure::{FailureEvent, FailureInfo},
    telemetry::TelemetryObservationConfig,
  },
  test_support::TestMailboxFactory,
};
use cellex_actor_std_rs::FailureEventHub;
use cellex_remote_core_rs::RemoteFailureNotifier;
use cellex_serialization_core_rs::{
  impl_type_key, InMemorySerializerRegistry, SerializationRouter, SerializedMessage, TypeBindingRegistry, TypeKey,
};
use cellex_serialization_json_rs::{shared_json_serializer, SerdeJsonSerializer, SERDE_JSON_SERIALIZER_ID};
use serde::{Deserialize, Serialize};
use serde_json::from_slice;

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
fn cluster_failure_bridge_triggers_telemetry_metrics() {
  let hub = FailureEventHub::new();
  let remote_hub = FailureEventHub::new();

  let (local_telemetry_impl, local_telemetry_events) = RecordingTelemetry::new();
  let local_telemetry = FailureTelemetryShared::new(local_telemetry_impl);
  let (local_metrics_impl, local_metrics_events) = RecordingMetricsSink::new();
  let local_metrics = MetricsSinkShared::new(local_metrics_impl);
  let mut local_observation = TelemetryObservationConfig::new().with_metrics_sink(local_metrics);
  local_observation.set_record_timing(true);

  let mut local_root: RootEscalationSink<TestMailboxFactory> = RootEscalationSink::new();
  local_root.set_telemetry(local_telemetry);
  local_root.set_observation_config(local_observation);
  let local_sink = Arc::new(Mutex::new(local_root));
  let local_sink_clone: Arc<Mutex<RootEscalationSink<TestMailboxFactory>>> = Arc::clone(&local_sink);
  let _local_subscription = hub.subscribe(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    let handle_result = local_sink_clone.lock().unwrap_or_else(|err| err.into_inner()).handle(info, false);
    assert!(handle_result.is_ok(), "local root sink should handle failure: {:?}", handle_result.err());
  }));

  let (remote_telemetry_impl, remote_telemetry_events) = RecordingTelemetry::new();
  let remote_telemetry = FailureTelemetryShared::new(remote_telemetry_impl);
  let (remote_metrics_impl, remote_metrics_events) = RecordingMetricsSink::new();
  let remote_metrics = MetricsSinkShared::new(remote_metrics_impl);
  let mut remote_observation = TelemetryObservationConfig::new().with_metrics_sink(remote_metrics);
  remote_observation.set_record_timing(true);

  let mut remote_root: RootEscalationSink<TestMailboxFactory> = RootEscalationSink::new();
  remote_root.set_telemetry(remote_telemetry);
  remote_root.set_observation_config(remote_observation);
  let remote_sink = Arc::new(Mutex::new(remote_root));
  let remote_sink_clone: Arc<Mutex<RootEscalationSink<TestMailboxFactory>>> = Arc::clone(&remote_sink);

  let mut remote_notifier = RemoteFailureNotifier::new(remote_hub);
  remote_notifier.set_handler(FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    let handle_result = remote_sink_clone.lock().unwrap_or_else(|err| err.into_inner()).handle(info, false);
    assert!(handle_result.is_ok(), "remote root sink should handle failure: {:?}", handle_result.err());
  }));

  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let failure = ActorFailure::from_message("cluster telemetry failure");
  let info = FailureInfo::new(ActorId(13), ActorPath::new(), failure);
  bridge.fan_out(FailureEvent::RootEscalated(info));

  let local_descriptions = local_telemetry_events.lock().unwrap_or_else(|err| err.into_inner()).clone();
  assert_eq!(local_descriptions, vec!["cluster telemetry failure".to_string()]);

  let local_metrics_values = local_metrics_events.lock().unwrap_or_else(|err| err.into_inner()).clone();
  assert!(local_metrics_values.contains(&MetricsEvent::TelemetryInvoked));
  assert!(local_metrics_values.iter().any(|event| matches!(event, MetricsEvent::TelemetryLatencyNanos(_))));

  let remote_descriptions = remote_telemetry_events.lock().unwrap_or_else(|err| err.into_inner()).clone();
  assert_eq!(remote_descriptions, vec!["cluster telemetry failure".to_string()]);

  let remote_metrics_values = remote_metrics_events.lock().unwrap_or_else(|err| err.into_inner()).clone();
  assert!(remote_metrics_values.contains(&MetricsEvent::TelemetryInvoked));
  assert!(remote_metrics_values.iter().any(|event| matches!(event, MetricsEvent::TelemetryLatencyNanos(_))));
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ClusterRouterPayload {
  value: String,
}

impl_type_key!(ClusterRouterPayload, "test.ClusterRouterPayload");

#[derive(Debug, Clone)]
struct RouterBehaviorFailure {
  message: SerializedMessage,
}

impl RouterBehaviorFailure {
  fn new(message: SerializedMessage) -> Self {
    Self { message }
  }

  fn serialized(&self) -> &SerializedMessage {
    &self.message
  }
}

impl BehaviorFailure for RouterBehaviorFailure {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn description(&self) -> Cow<'_, str> {
    let type_name = self.message.type_name.as_deref().unwrap_or("unknown");
    Cow::Owned(format!("cluster router failure for {type_name}"))
  }
}

fn decode_router_payload(router: &SerializationRouter, message: &SerializedMessage) -> Option<ClusterRouterPayload> {
  let type_key = message.type_name.as_deref()?;
  let serializer = router.resolve_or_fallback(type_key)?;
  let bytes = serializer.deserialize(message).ok()?;
  from_slice(&bytes).ok()
}

#[test]
fn cluster_serialization_router_roundtrip_with_fallback() -> TestResult {
  let bindings = TypeBindingRegistry::new();
  let registry = InMemorySerializerRegistry::new();
  let router = SerializationRouter::with_fallback(bindings, registry.clone(), Some(SERDE_JSON_SERIALIZER_ID));

  registry.register(shared_json_serializer()).map_err(|err| format!("シリアライザ登録に失敗しました: {err}"))?;

  let serializer = SerdeJsonSerializer::new();
  let payload = ClusterRouterPayload { value: "cluster route".to_string() };
  let expected = payload.value.clone();

  let serialized = serializer
    .serialize_value(Some(<ClusterRouterPayload as TypeKey>::type_key()), &payload)
    .map_err(|err| format!("シリアライズに失敗しました: {err}"))?;

  let hub = FailureEventHub::new();
  let remote_hub = FailureEventHub::new();
  let mut remote_notifier = RemoteFailureNotifier::new(remote_hub);

  let remote_captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
  let router_for_remote = router.clone();
  remote_notifier.set_handler(FailureEventListener::new({
    let remote_captured = Arc::clone(&remote_captured);
    move |event: FailureEvent| {
      let FailureEvent::RootEscalated(info) = event;
      if let Some(custom) = info.behavior_failure().as_any().downcast_ref::<RouterBehaviorFailure>() {
        if let Some(decoded) = decode_router_payload(&router_for_remote, custom.serialized()) {
          remote_captured.lock().unwrap_or_else(|err| err.into_inner()).replace(decoded.value);
        }
      }
    }
  }));

  let local_captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
  let router_for_local = router;
  let _subscription = hub.subscribe(FailureEventListener::new({
    let local_captured = Arc::clone(&local_captured);
    move |event: FailureEvent| {
      let FailureEvent::RootEscalated(info) = event;
      if let Some(custom) = info.behavior_failure().as_any().downcast_ref::<RouterBehaviorFailure>() {
        if let Some(decoded) = decode_router_payload(&router_for_local, custom.serialized()) {
          local_captured.lock().unwrap_or_else(|err| err.into_inner()).replace(decoded.value);
        }
      }
    }
  }));

  let bridge = ClusterFailureBridge::new(hub, remote_notifier);

  let failure = ActorFailure::new(RouterBehaviorFailure::new(serialized));
  let info = FailureInfo::new(ActorId(7), ActorPath::new(), failure);
  bridge.fan_out(FailureEvent::RootEscalated(info));

  let remote_value = lock(&remote_captured)?.clone().ok_or_else(|| "リモート側の復元結果がありません".to_string())?;
  assert_eq!(remote_value, expected);

  let local_value = lock(&local_captured)?.clone().ok_or_else(|| "ローカル側の復元結果がありません".to_string())?;
  assert_eq!(local_value, expected);
  Ok(())
}
