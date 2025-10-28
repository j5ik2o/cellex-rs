use cellex_utils_core_rs::sync::shared::SharedBound;

use super::metrics_event::MetricsEvent;

/// Abstract sink that receives metrics events emitted by the runtime.
pub trait MetricsSink: SharedBound + 'static {
  /// Records a metrics event.
  fn record(&self, event: MetricsEvent);
}
