use crate::api::actor::failure::BehaviorFailure;
use crate::api::supervision::supervisor::Supervisor;
use crate::api::supervision::supervisor::SupervisorDirective;

/// No-op supervisor implementation.
///
/// Returns `Resume` for all failures and continues processing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopSupervisor;

impl<M> Supervisor<M> for NoopSupervisor {
  /// Returns `Resume` for all failures.
  fn decide(&mut self, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    SupervisorDirective::Resume
  }
}
