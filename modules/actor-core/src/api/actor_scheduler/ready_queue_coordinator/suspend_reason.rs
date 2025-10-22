//! SuspendReason - Reasons for actor suspension

/// Reason for actor suspension
#[derive(Debug, Clone, PartialEq)]
pub enum SuspendReason {
  /// Suspended due to backpressure
  Backpressure,
  /// Suspended while awaiting external event
  AwaitExternal,
  /// Suspended due to rate limiting
  RateLimit,
  /// User-defined suspension reason
  UserDefined,
}
