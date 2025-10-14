use super::FailureInfo;

/// Failure event. Represents a failure that occurred within the actor system.
#[derive(Clone, Debug)]
pub enum FailureEvent {
  /// Failure escalated to root actor.
  /// Used when no further escalation is possible.
  RootEscalated(FailureInfo),
}
