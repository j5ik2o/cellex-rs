use crate::api::supervision::telemetry::failure_snapshot::FailureSnapshot;
use cellex_utils_core_rs::SharedBound;

/// Telemetry hook invoked whenever a failure reaches the root escalation sink.
pub trait FailureTelemetry: SharedBound {
  /// Called with the failure information before any event handlers/listeners run.
  fn on_failure(&self, snapshot: &FailureSnapshot);
}
