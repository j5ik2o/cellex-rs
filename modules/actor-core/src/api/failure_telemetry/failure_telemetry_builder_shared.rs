use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use cellex_utils_core_rs::Shared;

use super::failure_telemetry_shared::FailureTelemetryShared;
use super::telemetry_builder_fn::TelemetryBuilderFn;
use super::telemetry_context::TelemetryContext;

/// Shared wrapper around a failure telemetry builder function.
pub struct FailureTelemetryBuilderShared {
  inner: ArcShared<dyn TelemetryBuilderFn>,
}

impl FailureTelemetryBuilderShared {
  /// Creates a new shared telemetry builder from the provided closure.
  #[must_use]
  pub fn new<F>(builder: F) -> Self
  where
    F: Fn(&TelemetryContext) -> FailureTelemetryShared + SharedBound + 'static, {
    let shared = ArcShared::new(builder);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn TelemetryBuilderFn),
    }
  }

  /// Executes the builder to obtain a telemetry implementation.
  #[must_use]
  pub fn build(&self, ctx: &TelemetryContext) -> FailureTelemetryShared {
    self.inner.with_ref(|builder| builder.build(ctx))
  }
}

impl Clone for FailureTelemetryBuilderShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}
