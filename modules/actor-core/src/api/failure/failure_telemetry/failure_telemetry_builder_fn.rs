use cellex_utils_core_rs::sync::shared::SharedBound;

use super::{failure_telemetry_context::FailureTelemetryContext, failure_telemetry_shared::FailureTelemetryShared};

pub(crate) trait FailureTelemetryBuilderFn: SharedBound {
  fn build(&self, ctx: &FailureTelemetryContext) -> FailureTelemetryShared;
}

impl<F> FailureTelemetryBuilderFn for F
where
  F: Fn(&FailureTelemetryContext) -> FailureTelemetryShared + SharedBound,
{
  fn build(&self, ctx: &FailureTelemetryContext) -> FailureTelemetryShared {
    (self)(ctx)
  }
}
