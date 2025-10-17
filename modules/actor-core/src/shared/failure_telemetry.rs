use crate::internal::metrics::MetricsSinkShared;
use crate::{Extensions, FailureEvent, FailureInfo, FailureTelemetry};
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use cellex_utils_core_rs::Shared;

#[cfg(target_has_atomic = "ptr")]
type FailureEventHandlerFn = dyn Fn(&FailureInfo) + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type FailureEventHandlerFn = dyn Fn(&FailureInfo);

#[cfg(target_has_atomic = "ptr")]
type FailureEventListenerFn = dyn Fn(FailureEvent) + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type FailureEventListenerFn = dyn Fn(FailureEvent);

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
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn FailureTelemetry),
    }
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
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl core::ops::Deref for FailureTelemetryShared {
  type Target = dyn FailureTelemetry;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Context provided to telemetry builders.
pub struct TelemetryContext {
  metrics: Option<MetricsSinkShared>,
  extensions: Extensions,
}

impl TelemetryContext {
  /// Creates a new telemetry context with optional metrics sink information.
  #[must_use]
  pub const fn new(metrics: Option<MetricsSinkShared>, extensions: Extensions) -> Self {
    Self { metrics, extensions }
  }

  /// Returns the metrics sink associated with the context, if any.
  #[must_use]
  pub const fn metrics_sink(&self) -> Option<&MetricsSinkShared> {
    self.metrics.as_ref()
  }

  /// Returns the extension registry reference.
  #[must_use]
  pub const fn extensions(&self) -> &Extensions {
    &self.extensions
  }
}

trait TelemetryBuilderFn: SharedBound {
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

/// Shared wrapper for failure event handlers.
pub struct FailureEventHandlerShared {
  inner: ArcShared<FailureEventHandlerFn>,
}

impl FailureEventHandlerShared {
  /// Creates a new shared handler from a closure.
  #[must_use]
  pub fn new<F>(handler: F) -> Self
  where
    F: Fn(&FailureInfo) + SharedBound + 'static, {
    let shared = ArcShared::new(handler);
    Self {
      inner: shared.into_dyn(|inner| inner as &FailureEventHandlerFn),
    }
  }

  /// Wraps an existing shared handler reference.
  #[must_use]
  pub fn from_shared(inner: ArcShared<FailureEventHandlerFn>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handler.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<FailureEventHandlerFn> {
    self.inner
  }
}

impl Clone for FailureEventHandlerShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl core::ops::Deref for FailureEventHandlerShared {
  type Target = FailureEventHandlerFn;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Shared wrapper for failure event listeners.
pub struct FailureEventListenerShared {
  inner: ArcShared<FailureEventListenerFn>,
}

impl FailureEventListenerShared {
  /// Creates a new shared listener from a closure.
  #[must_use]
  pub fn new<F>(listener: F) -> Self
  where
    F: Fn(FailureEvent) + SharedBound + 'static, {
    let shared = ArcShared::new(listener);
    Self {
      inner: shared.into_dyn(|inner| inner as &FailureEventListenerFn),
    }
  }

  /// Wraps an existing shared listener.
  #[must_use]
  pub fn from_shared(inner: ArcShared<FailureEventListenerFn>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared listener.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<FailureEventListenerFn> {
    self.inner
  }
}

impl Clone for FailureEventListenerShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl core::ops::Deref for FailureEventListenerShared {
  type Target = FailureEventListenerFn;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
