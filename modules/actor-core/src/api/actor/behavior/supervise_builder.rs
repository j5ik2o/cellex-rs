use super::{Behavior, SupervisorStrategyConfig};
use crate::api::actor::behavior::supervisor_strategy::SupervisorStrategy;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::messaging::MetadataStorageMode;
use cellex_utils_core_rs::Element;

/// Builder for setting supervisor strategy.
pub struct SuperviseBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  pub(crate) behavior: Behavior<U, R>,
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
