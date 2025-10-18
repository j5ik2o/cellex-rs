use super::{metrics_event::MetricsEvent, metrics_sink::MetricsSink};

/// 何も記録しないノップ実装。
#[derive(Clone, Default)]
pub struct NoopMetricsSink;

impl MetricsSink for NoopMetricsSink {
  fn record(&self, _event: MetricsEvent) {
    // intentionally noop
  }
}
