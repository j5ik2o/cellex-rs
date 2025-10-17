#![cfg(feature = "std")]

use super::*;

use crate::api::actor::failure::ActorFailure;
use crate::api::identity::ActorId;
use crate::api::identity::ActorPath;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::failure::{EscalationStage, FailureMetadata};
use crate::api::supervision::telemetry::tracing_failure_telemetry::TracingFailureTelemetry;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::subscriber::with_default;
use tracing_subscriber::fmt;

#[test]
fn failure_snapshot_captures_core_fields() {
  let metadata = FailureMetadata::default()
    .with_component("runtime")
    .with_transport("loopback")
    .insert_tag("key", "value");
  let path = ActorPath::new().push_child(ActorId(42));
  let failure = ActorFailure::from_message("test failure");
  let info = FailureInfo::new_with_metadata(ActorId(42), path.clone(), failure.clone(), metadata.clone())
    .with_stage(EscalationStage::Escalated { hops: 2 });

  let snapshot = FailureSnapshot::from_failure_info(&info);

  assert_eq!(snapshot.actor(), ActorId(42));
  assert_eq!(snapshot.path(), &path);
  assert_eq!(snapshot.failure(), &failure);
  assert_eq!(snapshot.metadata(), &metadata);
  assert_eq!(snapshot.stage(), EscalationStage::Escalated { hops: 2 });
  assert_eq!(snapshot.description(), info.description().as_ref());

  let tags: Vec<_> = snapshot
    .tags()
    .iter()
    .map(|tag| (tag.key().to_string(), tag.value().to_string()))
    .collect();
  assert!(tags.contains(&("component".to_string(), "runtime".to_string())));
  assert!(tags.contains(&("transport".to_string(), "loopback".to_string())));
  assert!(tags.contains(&("key".to_string(), "value".to_string())));
}

struct CaptureWriter {
  buffer: Arc<Mutex<Vec<u8>>>,
}

impl Write for CaptureWriter {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    let mut guard = self.buffer.lock().unwrap();
    guard.extend_from_slice(buf);
    Ok(buf.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

#[test]
fn tracing_failure_telemetry_emits_error_log() {
  let metadata = FailureMetadata::default();
  let path = ActorPath::new();
  let failure = ActorFailure::from_message("log check");
  let info = FailureInfo::new_with_metadata(ActorId(7), path, failure, metadata.clone())
    .with_stage(EscalationStage::Escalated { hops: 3 });
  let snapshot = FailureSnapshot::from_failure_info(&info);

  let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
  let writer_source = buffer.clone();
  let subscriber = fmt::SubscriberBuilder::default()
    .with_writer(move || CaptureWriter {
      buffer: writer_source.clone(),
    })
    .with_ansi(false)
    .finish();

  let telemetry = TracingFailureTelemetry;
  with_default(subscriber, || {
    telemetry.on_failure(&snapshot);
  });

  let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
  assert!(output.contains("actor escalation reached root guardian"));
  assert!(output.contains("ActorId(7)"));
  assert!(output.contains("Escalated"));
  assert!(output.contains("hops: 3"));
}
