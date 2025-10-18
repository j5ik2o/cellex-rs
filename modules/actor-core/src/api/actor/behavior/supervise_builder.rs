use cellex_utils_core_rs::Element;

use super::{Behavior, SupervisorStrategyConfig};
use crate::api::{
  actor::behavior::supervisor_strategy::SupervisorStrategy,
  actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf},
  mailbox::PriorityEnvelope,
  messaging::{AnyMessage, MetadataStorageMode},
};

/// Builder for setting supervisor strategy.
pub struct SuperviseBuilder<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  pub(crate) behavior: Behavior<U, AR>,
}

impl<U, AR> SuperviseBuilder<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Sets supervisor strategy.
  pub fn with_strategy(mut self, strategy: SupervisorStrategy) -> Behavior<U, AR> {
    if let Behavior::Receive(state) = &mut self.behavior {
      state.supervisor = SupervisorStrategyConfig::from_strategy(strategy);
    }
    self.behavior
  }
}
