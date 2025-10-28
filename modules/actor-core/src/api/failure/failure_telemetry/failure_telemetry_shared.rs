use cellex_utils_core_rs::sync::{
  shared::{Shared, SharedBound},
  ArcShared,
};

use crate::api::failure::failure_telemetry::FailureTelemetry;

/// Shared wrapper around a [`FailureTelemetry`] implementation.
pub struct FailureTelemetryShared {
  inner: ArcShared<dyn FailureTelemetry>,
}

impl FailureTelemetryShared {
  /// Creates a new shared telemetry handle from a concrete implementation.
  #[must_use]
  pub fn new<T>(telemetry: T) -> Self
  where
    T: FailureTelemetry + SharedBound + 'static, {
    let shared = ArcShared::new(telemetry);
    Self { inner: shared.into_dyn(|inner| inner as &dyn FailureTelemetry) }
  }

  /// Wraps an existing shared telemetry handle.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn FailureTelemetry>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn FailureTelemetry> {
    self.inner
  }

  /// Executes the provided closure with a shared reference to the telemetry implementation.
  pub fn with_ref<R>(&self, f: impl FnOnce(&dyn FailureTelemetry) -> R) -> R {
    self.inner.with_ref(|inner| f(inner))
  }
}

impl Clone for FailureTelemetryShared {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl core::ops::Deref for FailureTelemetryShared {
  type Target = dyn FailureTelemetry;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
