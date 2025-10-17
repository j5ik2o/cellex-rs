use crate::api::actor::actor_failure::BehaviorFailure;
use crate::api::actor::behavior::supervisor_strategy::SupervisorStrategy;
use crate::api::supervision::supervisor::Supervisor;
use crate::api::supervision::supervisor::SupervisorDirective;

pub struct FixedDirectiveSupervisor {
  directive: SupervisorDirective,
}

impl FixedDirectiveSupervisor {
  pub fn new(strategy: SupervisorStrategy) -> Self {
    Self {
      directive: strategy.into(),
    }
  }
}

impl<M> Supervisor<M> for FixedDirectiveSupervisor {
  fn decide(&mut self, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    self.directive
  }
}
