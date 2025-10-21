use alloc::boxed::Box;

use cellex_utils_core_rs::{sync::ArcShared, Element};

use crate::{
  api::{
    actor::{
      actor_context::ActorContext,
      actor_failure::ActorFailure,
      behavior::{
        behavior_directive::BehaviorDirective, supervisor_strategy_config::SupervisorStrategyConfig, ReceiveFn,
        SignalFn,
      },
    },
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf},
    messaging::MetadataStorageMode,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Struct that holds the internal state of Behavior.
pub struct BehaviorState<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  handler:               Box<ReceiveFn<U, AR>>,
  pub(super) supervisor: SupervisorStrategyConfig,
  signal_handler:        Option<ArcShared<SignalFn<U, AR>>>,
}

impl<U, AR> BehaviorState<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  pub fn new(handler: Box<ReceiveFn<U, AR>>, supervisor: SupervisorStrategyConfig) -> Self {
    Self { handler, supervisor, signal_handler: None }
  }

  pub fn handle(
    &mut self,
    ctx: &mut ActorContext<'_, '_, U, AR>,
    message: U,
  ) -> Result<BehaviorDirective<U, AR>, ActorFailure> {
    (self.handler)(ctx, message)
  }

  pub(super) fn signal_handler(&self) -> Option<ArcShared<SignalFn<U, AR>>> {
    self.signal_handler.clone()
  }

  pub(super) fn set_signal_handler(&mut self, handler: ArcShared<SignalFn<U, AR>>) {
    self.signal_handler = Some(handler);
  }
}
