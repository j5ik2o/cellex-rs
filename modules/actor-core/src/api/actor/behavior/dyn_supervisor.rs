use alloc::boxed::Box;

use cellex_utils_core_rs::Element;

use crate::api::{
  actor::actor_failure::BehaviorFailure,
  supervision::supervisor::{Supervisor, SupervisorDirective},
};

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
