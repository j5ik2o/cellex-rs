use crate::{MetricsEvent, MetricsSinkShared};

/// Telemetry 呼び出しの観測設定。
#[derive(Clone, Default, Debug)]
pub struct TelemetryObservationConfig {
  metrics: Option<MetricsSinkShared>,
  record_timing: bool,
}

impl TelemetryObservationConfig {
  /// 新しい設定を生成する。
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// メトリクスシンクを設定する。
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics = sink;
  }

  /// メトリクスシンクを持つ設定を返す（builder 用）。
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics = Some(sink);
    self
  }

  /// 現在のメトリクスシンクを参照する。
  #[must_use]
  pub fn metrics_sink(&self) -> Option<&MetricsSinkShared> {
    self.metrics.as_ref()
  }

  /// 呼び出し時間計測の有効／無効を設定する。
  pub fn set_record_timing(&mut self, enabled: bool) {
    self.record_timing = enabled;
  }

  /// 呼び出し時間を記録するかどうか。
  #[must_use]
  pub fn should_record_timing(&self) -> bool {
    self.record_timing
  }

  /// Telemetry 呼び出し後に観測結果を記録する。
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
