use crate::api::actor::BehaviorFailure;
use crate::api::supervision::{NoopSupervisor, Supervisor, SupervisorDirective};
use alloc::boxed::Box;
use cellex_utils_core_rs::Element;

/// Supervisor strategy configuration (internal representation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupervisorStrategyConfig {
  /// Default strategy (NoopSupervisor)
  Default,
  /// Fixed strategy
  Fixed(SupervisorStrategy),
}

impl SupervisorStrategyConfig {
  pub(crate) const fn default() -> Self {
    SupervisorStrategyConfig::Default
  }

  pub(crate) const fn from_strategy(strategy: SupervisorStrategy) -> Self {
    SupervisorStrategyConfig::Fixed(strategy)
  }

  pub(crate) fn as_supervisor<M>(&self) -> DynSupervisor<M>
  where
    M: Element, {
    let inner: Box<dyn Supervisor<M>> = match self {
      SupervisorStrategyConfig::Default => Box::new(NoopSupervisor),
      SupervisorStrategyConfig::Fixed(strategy) => Box::new(FixedDirectiveSupervisor::new(*strategy)),
    };
    DynSupervisor::new(inner)
  }
}

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

struct FixedDirectiveSupervisor {
  directive: SupervisorDirective,
}

impl FixedDirectiveSupervisor {
  fn new(strategy: SupervisorStrategy) -> Self {
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

/// Dynamic supervisor implementation (internal type).
pub(crate) struct DynSupervisor<M>
where
  M: Element, {
  inner: Box<dyn Supervisor<M>>,
}

impl<M> DynSupervisor<M>
where
  M: Element,
{
  pub(crate) fn new(inner: Box<dyn Supervisor<M>>) -> Self {
    Self { inner }
  }
}

impl<M> Supervisor<M> for DynSupervisor<M>
where
  M: Element,
{
  fn before_handle(&mut self) {
    self.inner.before_handle();
  }

  fn after_handle(&mut self) {
    self.inner.after_handle();
  }

  fn decide(&mut self, error: &dyn BehaviorFailure) -> SupervisorDirective {
    self.inner.decide(error)
  }
}
