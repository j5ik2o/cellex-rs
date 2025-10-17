use crate::api::supervision::supervisor::SupervisorDirective;

/// Types of supervisor strategies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorStrategy {
  /// Restart the actor
  Restart,
  /// Stop the actor
  Stop,
  /// Ignore the error and continue processing
  Resume,
  /// Escalate to parent
  Escalate,
}

impl From<SupervisorStrategy> for SupervisorDirective {
  fn from(value: SupervisorStrategy) -> Self {
    match value {
      SupervisorStrategy::Restart => SupervisorDirective::Restart,
      SupervisorStrategy::Stop => SupervisorDirective::Stop,
      SupervisorStrategy::Resume => SupervisorDirective::Resume,
      SupervisorStrategy::Escalate => SupervisorDirective::Escalate,
    }
  }
}
