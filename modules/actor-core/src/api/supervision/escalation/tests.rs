use crate::api::actor::actor_failure::ActorFailure;
use crate::api::actor::ActorId;
use crate::api::actor::ActorPath;
use crate::api::supervision::escalation::escalation_sink::EscalationSink;
use crate::api::supervision::escalation::root_escalation_sink::RootEscalationSink;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::telemetry::FailureSnapshot;
use crate::api::supervision::telemetry::FailureTelemetry;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::mailbox::test_support::TestMailboxRuntime;
use crate::internal::metrics::MetricsEvent;
use crate::internal::metrics::MetricsSink;
use crate::internal::metrics::MetricsSinkShared;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct RecordingTelemetry {
  events: Arc<Mutex<Vec<FailureSnapshot>>>,
}

impl RecordingTelemetry {
  fn new() -> (Self, Arc<Mutex<Vec<FailureSnapshot>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    (Self { events: events.clone() }, events)
  }
}

impl FailureTelemetry for RecordingTelemetry {
  fn on_failure(&self, snapshot: &FailureSnapshot) {
    let mut guard = self.events.lock().unwrap();
    guard.push(snapshot.clone());
  }
}

#[derive(Clone, Debug)]
struct DummyMessage;

#[test]
fn root_escalation_sink_invokes_telemetry() {
  let (telemetry_impl, events) = RecordingTelemetry::new();
  let telemetry_shared = FailureTelemetryShared::new(telemetry_impl);

  let mut sink: RootEscalationSink<DummyMessage, TestMailboxRuntime> = RootEscalationSink::new();
  sink.set_telemetry(telemetry_shared);

  let failure = ActorFailure::from_message("boom");
  let info = FailureInfo::new(ActorId(1), ActorPath::new(), failure);

  sink.handle(info.clone(), false).expect("sink handle");

  let guard = events.lock().unwrap();
  assert_eq!(guard.len(), 1);
  assert_eq!(guard[0].actor(), ActorId(1));
  assert_eq!(guard[0].description(), info.description());
}

#[test]
fn telemetry_default_is_noop() {
  let mut sink: RootEscalationSink<DummyMessage, TestMailboxRuntime> = RootEscalationSink::new();
  let failure = ActorFailure::from_message("boom");
  let info = FailureInfo::new(ActorId(7), ActorPath::new(), failure);

  // Should not panic even though telemetry does nothing by default.
  sink.handle(info, false).expect("sink handle");
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
    let mut guard = self.events.lock().unwrap();
    guard.push(event);
  }
}

#[test]
fn root_escalation_sink_records_metrics() {
  let (metrics_sink_impl, metrics_events) = RecordingMetricsSink::new();
  let mut observation = TelemetryObservationConfig::new().with_metrics_sink(MetricsSinkShared::new(metrics_sink_impl));
  observation.set_record_timing(true);

  let mut sink: RootEscalationSink<DummyMessage, TestMailboxRuntime> = RootEscalationSink::new();
  sink.set_observation_config(observation);

  let failure = ActorFailure::from_message("boom");
  let info = FailureInfo::new(ActorId(9), ActorPath::new(), failure);

  sink.handle(info, false).expect("sink handle");

  let guard = metrics_events.lock().unwrap();
  assert!(guard.contains(&MetricsEvent::TelemetryInvoked));
  assert!(guard
    .iter()
    .any(|event| matches!(event, MetricsEvent::TelemetryLatencyNanos(_))));
}
