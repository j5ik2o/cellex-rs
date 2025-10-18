use cellex_utils_core_rs::sync::SharedBound;

use super::{failure_telemetry_shared::FailureTelemetryShared, telemetry_context::TelemetryContext};

pub(crate) trait TelemetryBuilderFn: SharedBound {
  fn build(&self, ctx: &TelemetryContext) -> FailureTelemetryShared;
}

impl<F> TelemetryBuilderFn for F
where
  F: Fn(&TelemetryContext) -> FailureTelemetryShared + SharedBound,
{
  fn build(&self, ctx: &TelemetryContext) -> FailureTelemetryShared {
    (self)(ctx)
  }
}
