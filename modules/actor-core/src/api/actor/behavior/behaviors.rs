use super::{Behavior, BehaviorDirective, SupervisorStrategy, SupervisorStrategyConfig};
use crate::api::actor::context::Context;
use crate::api::actor::ActorFailure;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::PriorityEnvelope;
use crate::{DynMessage, MetadataStorageMode};
use cellex_utils_core_rs::Element;

/// Behavior DSL builder.
pub struct Behaviors;

impl Behaviors {
  /// Constructs Behavior with specified message receive handler.
  #[must_use]
  pub fn receive<U, R, F>(handler: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone,
    MailboxConcurrencyOf<R>: MetadataStorageMode,
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<BehaviorDirective<U, R>, ActorFailure> + 'static,
  {
    Behavior::receive(handler)
  }

  /// Returns a directive to maintain current Behavior.
  #[must_use]
  pub const fn same<U, R>() -> BehaviorDirective<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone, {
    BehaviorDirective::Same
  }

  /// Constructs Behavior with a handler that receives only the message.
  #[must_use]
  pub fn receive_message<U, R, F>(handler: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone,
    MailboxConcurrencyOf<R>: MetadataStorageMode,
    F: FnMut(U) -> Result<BehaviorDirective<U, R>, ActorFailure> + 'static, {
    Behavior::receive_message(handler)
  }

  /// Returns a directive to transition to a new Behavior.
  #[must_use]
  pub const fn transition<U, R>(behavior: Behavior<U, R>) -> BehaviorDirective<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone, {
    BehaviorDirective::Become(behavior)
  }

  /// Returns a directive to transition to stopped state.
  #[must_use]
  pub const fn stopped<U, R>() -> BehaviorDirective<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone, {
    BehaviorDirective::Become(Behavior::stopped())
  }

  /// Creates a builder to set supervisor strategy on Behavior.
  #[must_use]
  pub const fn supervise<U, R>(behavior: Behavior<U, R>) -> SuperviseBuilder<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone, {
    SuperviseBuilder { behavior }
  }

  /// Executes setup processing to generate Behavior.
  pub fn setup<U, R, F>(init: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + 'static,
    MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<R>: Clone,
    MailboxConcurrencyOf<R>: MetadataStorageMode,
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Result<Behavior<U, R>, ActorFailure> + 'static, {
    Behavior::setup(init)
  }
}

/// Builder for setting supervisor strategy.
pub struct SuperviseBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  behavior: Behavior<U, R>,
}

impl<U, R> SuperviseBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Sets supervisor strategy.
  pub fn with_strategy(mut self, strategy: SupervisorStrategy) -> Behavior<U, R> {
    if let Behavior::Receive(state) = &mut self.behavior {
      state.supervisor = SupervisorStrategyConfig::from_strategy(strategy);
    }
    self.behavior
  }
}
