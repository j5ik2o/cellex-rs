use cellex_utils_core_rs::Element;

use super::{Behavior, BehaviorDirective};
use crate::api::{
  actor::{actor_context::ActorContext, actor_failure::ActorFailure, behavior::supervise_builder::SuperviseBuilder},
  actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf},
  mailbox::PriorityEnvelope,
  messaging::{DynMessage, MetadataStorageMode},
};

/// Behavior DSL builder.
pub struct Behaviors;

impl Behaviors {
  /// Constructs Behavior with specified message receive handler.
  #[must_use]
  pub fn receive<U, AR, F>(handler: F) -> Behavior<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone,
    MailboxConcurrencyOf<AR>: MetadataStorageMode,
    F: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, U) -> Result<BehaviorDirective<U, AR>, ActorFailure>
      + 'static, {
    Behavior::receive(handler)
  }

  /// Returns a directive to maintain current Behavior.
  #[must_use]
  pub const fn same<U, AR>() -> BehaviorDirective<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone, {
    BehaviorDirective::Same
  }

  /// Constructs Behavior with a handler that receives only the message.
  #[must_use]
  pub fn receive_message<U, AR, F>(handler: F) -> Behavior<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone,
    MailboxConcurrencyOf<AR>: MetadataStorageMode,
    F: FnMut(U) -> Result<BehaviorDirective<U, AR>, ActorFailure> + 'static, {
    Behavior::receive_message(handler)
  }

  /// Returns a directive to transition to a new Behavior.
  #[must_use]
  pub const fn transition<U, AR>(behavior: Behavior<U, AR>) -> BehaviorDirective<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone, {
    BehaviorDirective::Become(behavior)
  }

  /// Returns a directive to transition to stopped state.
  #[must_use]
  pub const fn stopped<U, AR>() -> BehaviorDirective<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone, {
    BehaviorDirective::Become(Behavior::stopped())
  }

  /// Creates a builder to set supervisor strategy on Behavior.
  #[must_use]
  pub const fn supervise<U, AR>(behavior: Behavior<U, AR>) -> SuperviseBuilder<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone, {
    SuperviseBuilder { behavior }
  }

  /// Executes setup processing to generate Behavior.
  pub fn setup<U, AR, F>(init: F) -> Behavior<U, AR>
  where
    U: Element,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
    MailboxSignalOf<AR>: Clone,
    MailboxConcurrencyOf<AR>: MetadataStorageMode,
    F: for<'r, 'ctx> Fn(&mut ActorContext<'r, 'ctx, U, AR>) -> Result<Behavior<U, AR>, ActorFailure> + 'static, {
    Behavior::setup(init)
  }
}
