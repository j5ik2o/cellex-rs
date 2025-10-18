use super::guardian_strategy::GuardianStrategy;
use crate::api::{
  actor::{actor_failure::BehaviorFailure, ActorId},
  mailbox::MailboxFactory,
  supervision::supervisor::SupervisorDirective,
};

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

impl<MF> GuardianStrategy<MF> for AlwaysRestart
where
  MF: MailboxFactory,
{
  fn decide(&mut self, _actor: ActorId, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    SupervisorDirective::Restart
  }
}
