use crate::{
  ActorRuntime, Behavior, DynMessage, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf, MetadataStorageMode,
  PriorityEnvelope,
};
use cellex_utils_core_rs::Element;

/// State transition directive after user message processing.
pub enum BehaviorDirective<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode, {
  /// Maintain the same Behavior
  Same,
  /// Transition to a new Behavior
  Become(Behavior<U, R>),
}
