//! Shared actor abstractions used across API and internal layers.

use cellex_utils_core_rs::collections::Element;

use crate::{
  api::{
    actor::{actor_context::ActorContext, actor_failure::ActorFailure, behavior::SupervisorStrategyConfig},
    actor_runtime::{ActorRuntime, MailboxQueueOf, MailboxSignalOf},
    mailbox::messages::SystemMessage,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Bridge trait for typed message handlers.
///
/// This trait abstracts the core actor message handling interface
/// needed by the internal layer, allowing API layer implementations
/// to provide typed message handling without creating circular dependencies.
pub trait TypedHandlerBridge<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  /// Handle a user message.
  ///
  /// # Errors
  /// Returns [`ActorFailure`] when message handling fails.
  fn handle_user(&mut self, ctx: &mut ActorContext<'_, '_, U, AR>, message: U) -> Result<(), ActorFailure>;

  /// Handle a system message.
  ///
  /// # Errors
  /// Returns [`ActorFailure`] when message handling fails.
  fn handle_system(
    &mut self,
    ctx: &mut ActorContext<'_, '_, U, AR>,
    message: SystemMessage,
  ) -> Result<(), ActorFailure>;

  /// Get supervisor configuration.
  fn supervisor_config(&self) -> SupervisorStrategyConfig;
}
