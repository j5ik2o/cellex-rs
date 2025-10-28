use cellex_utils_core_rs::sync::shared::SharedBound;

/// Clock abstraction used to measure suspension durations in a platform-agnostic way.
pub trait SuspensionClock: SharedBound + 'static {
  /// Returns a monotonically increasing timestamp in nanoseconds.
  /// Implementations may return `None` if measurement is unavailable.
  fn now(&self) -> Option<u64>;
}
