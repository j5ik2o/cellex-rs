use crate::api::metrics::{MetricsEvent, MetricsSinkShared};

/// Configuration controlling telemetry observations.
#[derive(Clone, Default, Debug)]
pub struct TelemetryObservationConfig {
  metrics:       Option<MetricsSinkShared>,
  record_timing: bool,
}

impl TelemetryObservationConfig {
  /// Creates a new configuration instance.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the metrics sink used to record observations.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics = sink;
  }

  /// Returns a configuration with the provided metrics sink set (builder-style).
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics = Some(sink);
    self
  }

  /// Returns the currently configured metrics sink, if any.
  #[must_use]
  pub fn metrics_sink(&self) -> Option<&MetricsSinkShared> {
    self.metrics.as_ref()
  }

  /// Enables or disables recording of call duration.
  pub fn set_record_timing(&mut self, enabled: bool) {
    self.record_timing = enabled;
  }

  /// Indicates whether call duration should be recorded.
  #[must_use]
  pub fn should_record_timing(&self) -> bool {
    self.record_timing
  }

  /// Records telemetry observations after a call completes.
  pub fn observe(&self, elapsed: Option<core::time::Duration>) {
    if let Some(metrics) = &self.metrics {
      metrics.with_ref(|sink| {
        sink.record(MetricsEvent::TelemetryInvoked);
        if let Some(duration) = elapsed {
          let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
          sink.record(MetricsEvent::TelemetryLatencyNanos(nanos));
        }
      });
    }
  }
}
