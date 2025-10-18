use cellex_utils_core_rs::Element;

use crate::api::{
  actor::behavior::Behavior,
  actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf},
  mailbox::PriorityEnvelope,
  messaging::{DynMessage, MetadataStorageMode},
};

/// State transition directive after user message processing.
pub enum BehaviorDirective<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  /// Maintain the same Behavior
  Same,
  /// Transition to a new Behavior
  Become(Behavior<U, AR>),
}
