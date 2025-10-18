use crate::api::{
  actor::{actor_failure::BehaviorFailure, behavior::supervisor_strategy::SupervisorStrategy},
  supervision::supervisor::{Supervisor, SupervisorDirective},
};

pub struct FixedDirectiveSupervisor {
  directive: SupervisorDirective,
}

impl FixedDirectiveSupervisor {
  pub fn new(strategy: SupervisorStrategy) -> Self {
    Self { directive: strategy.into() }
  }
}

impl<M> Supervisor<M> for FixedDirectiveSupervisor {
  fn decide(&mut self, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    self.directive
  }
}
