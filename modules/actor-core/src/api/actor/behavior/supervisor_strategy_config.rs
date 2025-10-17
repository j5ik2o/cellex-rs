use crate::api::actor::behavior::dyn_supervisor::DynSupervisor;
use crate::api::actor::behavior::fixed_directive_supervisor::FixedDirectiveSupervisor;
use crate::api::actor::behavior::supervisor_strategy::SupervisorStrategy;
use crate::api::supervision::supervisor::NoopSupervisor;
use crate::api::supervision::supervisor::Supervisor;
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
