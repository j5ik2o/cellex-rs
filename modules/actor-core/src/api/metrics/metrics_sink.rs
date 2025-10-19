use super::metrics_event::MetricsEvent;

/// Abstract sink that receives metrics events emitted by the runtime.
pub trait MetricsSink: Send + Sync + 'static {
  /// Records a metrics event.
  fn record(&self, event: MetricsEvent);
}
