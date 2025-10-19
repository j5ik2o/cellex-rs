use super::{metrics_event::MetricsEvent, metrics_sink::MetricsSink};

/// No-op metrics sink that intentionally records nothing.
#[derive(Clone, Default)]
pub struct NoopMetricsSink;

impl MetricsSink for NoopMetricsSink {
  fn record(&self, _event: MetricsEvent) {
    // intentionally noop
  }
}
