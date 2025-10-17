use super::metrics_event::MetricsEvent;

/// ランタイムがメトリクスを発行するための抽象シンク。
pub trait MetricsSink: Send + Sync + 'static {
  /// メトリクスイベントを記録する。
  fn record(&self, event: MetricsEvent);
}
