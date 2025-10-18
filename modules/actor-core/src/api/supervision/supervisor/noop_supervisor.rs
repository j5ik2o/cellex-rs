use crate::api::{
  actor::actor_failure::BehaviorFailure,
  supervision::supervisor::{Supervisor, SupervisorDirective},
};

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
