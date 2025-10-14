#![cfg_attr(not(feature = "std"), allow(unused_imports))]

#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;

use crate::FailureInfo;
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};

/// Telemetry hook invoked whenever a failure reaches the root escalation sink.
pub trait FailureTelemetry: SharedBound {
  /// Called with the failure information before any event handlers/listeners run.
  fn on_failure(&self, info: &FailureInfo);
}

/// Telemetry implementation that performs no side effects.
#[derive(Default, Clone, Copy)]
pub struct NullFailureTelemetry;

impl FailureTelemetry for NullFailureTelemetry {
  fn on_failure(&self, _info: &FailureInfo) {}
}

/// Returns a shared handle to the null telemetry implementation.
pub fn null_failure_telemetry() -> ArcShared<dyn FailureTelemetry> {
  ArcShared::from_arc(Arc::new(NullFailureTelemetry))
}

#[cfg(feature = "std")]
/// Telemetry implementation that emits tracing events.
pub struct TracingFailureTelemetry;

#[cfg(feature = "std")]
impl FailureTelemetry for TracingFailureTelemetry {
  fn on_failure(&self, info: &FailureInfo) {
    tracing::error!(
      actor = ?info.actor,
      reason = %info.description(),
      path = ?info.path.segments(),
      "actor escalation reached root guardian"
    );
  }
}

#[cfg(feature = "std")]
/// Returns a shared handle to the tracing-based telemetry implementation.
pub fn tracing_failure_telemetry() -> ArcShared<dyn FailureTelemetry> {
  ArcShared::from_arc(Arc::new(TracingFailureTelemetry))
}
