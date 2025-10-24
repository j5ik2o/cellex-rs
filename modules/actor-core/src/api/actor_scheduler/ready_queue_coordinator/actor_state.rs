//! ActorState - Actor execution states

/// Actor execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorState {
  /// Actor is actively processing messages
  Running,
  /// Actor is suspended
  Suspended,
  /// Actor is in the process of stopping
  Stopping,
  /// Actor has stopped
  Stopped,
}
