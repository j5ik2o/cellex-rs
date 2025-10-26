use cellex_utils_core_rs::collections::Element;

use crate::{
  api::{
    actor::behavior::Behavior,
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf},
    messaging::MetadataStorageMode,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// State transition directive after user message processing.
pub enum BehaviorDirective<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  /// Maintain the same Behavior
  Same,
  /// Transition to a new Behavior
  Become(Behavior<U, AR>),
}
