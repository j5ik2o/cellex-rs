#[cfg(feature = "std")]
use crate::api::failure::failure_telemetry::{
  failure_snapshot::FailureSnapshot, failure_telemetry::FailureTelemetry, FailureTelemetryShared,
};

#[cfg(feature = "std")]
/// Telemetry implementation that emits tracing events.
pub struct TracingFailureTelemetry;

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
/// Returns a shared handle to the tracing-based telemetry implementation.
#[must_use]
pub fn tracing_failure_telemetry() -> FailureTelemetryShared {
  FailureTelemetryShared::new(TracingFailureTelemetry)
}
