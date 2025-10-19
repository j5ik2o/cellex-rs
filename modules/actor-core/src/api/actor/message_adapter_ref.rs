use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};

use crate::api::{
  actor::{actor_context::AdapterFn, actor_ref::ActorRef},
  actor_runtime::{ActorRuntime, MailboxQueueOf, MailboxSignalOf},
  mailbox::messages::PriorityEnvelope,
  messaging::AnyMessage,
};

/// Reference to a message adapter.
#[derive(Clone)]
pub struct MessageAdapterRef<Ext, U, AR>
where
  Ext: Element,
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  target:  ActorRef<U, AR>,
  adapter: ArcShared<AdapterFn<Ext, U>>,
}

impl<Ext, U, AR> MessageAdapterRef<Ext, U, AR>
where
  Ext: Element,
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
{
  pub(crate) fn new(target: ActorRef<U, AR>, adapter: ArcShared<AdapterFn<Ext, U>>) -> Self {
    Self { target, adapter }
  }

  /// Converts an external message and sends it to the target actor.
  pub fn tell(&self, message: Ext) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let mapped = (self.adapter)(message);
    self.target.tell(mapped)
  }

  /// Converts an external message and sends it to the target actor with the specified priority.
  pub fn tell_with_priority(&self, message: Ext, priority: i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let mapped = (self.adapter)(message);
    self.target.tell_with_priority(mapped, priority)
  }

  /// Gets a reference to the target actor.
  #[must_use]
  pub fn target(&self) -> &ActorRef<U, AR> {
    &self.target
  }
}
