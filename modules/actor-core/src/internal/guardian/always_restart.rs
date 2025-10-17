use crate::api::actor::failure::BehaviorFailure;
use crate::api::identity::ActorId;
use crate::api::mailbox::MailboxFactory;
use crate::api::supervision::supervisor::SupervisorDirective;
use cellex_utils_core_rs::Element;

use super::guardian_strategy::GuardianStrategy;

/// Simplest strategy: Always instruct Restart.
///
/// Supervisor strategy that always instructs actor restart regardless of error type.
/// Suitable when expecting automatic recovery from temporary failures.
///
/// # Example Usage
/// ```ignore
/// let strategy = AlwaysRestart;
/// // A guardian using this strategy will always attempt to restart
/// // child actors when they fail
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysRestart;

impl<M, R> GuardianStrategy<M, R> for AlwaysRestart
where
  M: Element,
  R: MailboxFactory,
{
  fn decide(&mut self, _actor: ActorId, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    SupervisorDirective::Restart
  }
}
