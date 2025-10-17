use alloc::boxed::Box;

use crate::api::actor::behavior::behavior_directive::BehaviorDirective;
use crate::api::actor::behavior::supervisor_strategy_config::SupervisorStrategyConfig;
use crate::api::actor::behavior::{ReceiveFn, SignalFn};
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::PriorityEnvelope;
use crate::{ActorFailure, Context, DynMessage, MetadataStorageMode};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

/// Struct that holds the internal state of Behavior.
pub struct BehaviorState<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  handler: Box<ReceiveFn<U, R>>,
  pub(super) supervisor: SupervisorStrategyConfig,
  signal_handler: Option<ArcShared<SignalFn<U, R>>>,
}

impl<U, R> BehaviorState<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  pub fn new(handler: Box<ReceiveFn<U, R>>, supervisor: SupervisorStrategyConfig) -> Self {
    Self {
      handler,
      supervisor,
      signal_handler: None,
    }
  }

  pub fn handle(
    &mut self,
    ctx: &mut Context<'_, '_, U, R>,
    message: U,
  ) -> Result<BehaviorDirective<U, R>, ActorFailure> {
    (self.handler)(ctx, message)
  }

  pub(super) fn signal_handler(&self) -> Option<ArcShared<SignalFn<U, R>>> {
    self.signal_handler.clone()
  }

  pub(super) fn set_signal_handler(&mut self, handler: ArcShared<SignalFn<U, R>>) {
    self.signal_handler = Some(handler);
  }
}
