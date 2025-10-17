use crate::api::actor::behavior::Behavior;
use crate::api::actor_runtime::ActorRuntime;
use crate::api::actor_runtime::MailboxConcurrencyOf;
use crate::api::actor_runtime::MailboxQueueOf;
use crate::api::actor_runtime::MailboxSignalOf;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::messaging::MetadataStorageMode;
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
