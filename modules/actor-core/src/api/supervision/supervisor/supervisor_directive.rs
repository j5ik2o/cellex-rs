/// Action returned by the supervisor.
///
/// Instructs how the supervisor should respond when an actor fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorDirective {
  /// Stop the actor.
  Stop,
  /// Ignore the error and continue processing.
  Resume,
  /// Restart the actor.
  Restart,
  /// Escalate to parent.
  Escalate,
}
