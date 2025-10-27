use crate::api::actor_scheduler::InvokeResult;

/// Builder used by `ActorCell` to communicate scheduling outcomes.
#[derive(Default)]
pub struct ActorInvokeOutcome {
  result: Option<InvokeResult>,
}

impl ActorInvokeOutcome {
  #[must_use]
  pub const fn new() -> Self {
    Self { result: None }
  }

  pub fn set(&mut self, result: InvokeResult) {
    self.result = Some(result);
  }

  pub fn is_set(&self) -> bool {
    self.result.is_some()
  }

  pub fn into_result(self) -> Option<InvokeResult> {
    self.result
  }
}
