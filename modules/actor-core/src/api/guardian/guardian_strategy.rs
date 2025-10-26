use crate::{
  api::{
    actor::{actor_failure::BehaviorFailure, ActorId},
    supervision::supervisor::SupervisorDirective,
  },
  shared::mailbox::MailboxFactory,
};

/// Supervisor strategy. Corresponds to protoactor-go's Strategy.
///
/// Trait that defines the strategy applied when an actor fails.
/// Determines how the parent actor (guardian) handles child actor failures.
///
/// # Type Parameters
/// - `MF`: Factory type that generates mailboxes
pub trait GuardianStrategy<MF>: Send + 'static
where
  MF: MailboxFactory, {
  /// Determines the handling policy when an actor fails.
  ///
  /// # Arguments
  /// - `actor`: ID of the failed actor
  /// - `error`: Detailed information about the error that occurred
  ///
  /// # Returns
  /// Supervisor directive (Restart, Stop, Resume, Escalate, etc.)
  fn decide(&mut self, actor: ActorId, error: &dyn BehaviorFailure) -> SupervisorDirective;

  /// Hook called before actor startup.
  ///
  /// Default implementation does nothing. Override if needed.
  ///
  /// # Arguments
  /// - `_actor`: ID of the actor being started
  fn before_start(&mut self, _actor: ActorId) {}

  /// Hook called after actor restart.
  ///
  /// Default implementation does nothing. Override if needed.
  ///
  /// # Arguments
  /// - `_actor`: ID of the restarted actor
  fn after_restart(&mut self, _actor: ActorId) {}
}
