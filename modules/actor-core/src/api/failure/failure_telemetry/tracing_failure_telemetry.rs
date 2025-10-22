#[cfg(feature = "tracing-support")]
use crate::api::failure::failure_telemetry::{
  failure_snapshot::FailureSnapshot, failure_telemetry::FailureTelemetry, FailureTelemetryShared,
};

/// Telemetry implementation that emits tracing events.
#[cfg(feature = "tracing-support")]
pub struct TracingFailureTelemetry;

#[cfg(feature = "tracing-support")]
impl FailureTelemetry for TracingFailureTelemetry {
  fn on_failure(&self, snapshot: &FailureSnapshot) {
    tracing::error!(
      actor = ?snapshot.actor(),
      reason = %snapshot.description(),
      path = ?snapshot.path().segments(),
      stage = ?snapshot.stage(),
      "actor escalation reached root guardian"
    );
  }
}

/// Returns a shared handle to the tracing-based telemetry implementation.
#[cfg(feature = "tracing-support")]
#[must_use]
pub fn tracing_failure_telemetry() -> FailureTelemetryShared {
  FailureTelemetryShared::new(TracingFailureTelemetry)
}
