use crate::api::{extensions::Extensions, metrics::MetricsSinkShared};

/// Context provided to telemetry builders.
pub struct TelemetryContext {
  metrics:    Option<MetricsSinkShared>,
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
