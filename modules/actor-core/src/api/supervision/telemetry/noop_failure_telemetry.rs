use spin::Once;

use crate::api::{
  failure_telemetry::FailureTelemetryShared,
  supervision::telemetry::{failure_snapshot::FailureSnapshot, failure_telemetry::FailureTelemetry},
};

/// Telemetry implementation that performs no side effects.
#[derive(Default, Clone, Copy)]
pub struct NoopFailureTelemetry;

impl FailureTelemetry for NoopFailureTelemetry {
  fn on_failure(&self, _snapshot: &FailureSnapshot) {}
}

/// Returns a shared handle to the no-op telemetry implementation.
pub fn noop_failure_telemetry_shared() -> FailureTelemetryShared {
  #[cfg(target_has_atomic = "ptr")]
  {
    static INSTANCE: Once<FailureTelemetryShared> = Once::new();
    INSTANCE.call_once(|| FailureTelemetryShared::new(NoopFailureTelemetry)).clone()
  }

  #[cfg(not(target_has_atomic = "ptr"))]
  {
    FailureTelemetryShared::new(NoopFailureTelemetry)
  }
}
