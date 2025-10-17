use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::MailboxProducer;
use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::RuntimeBound;
use cellex_utils_core_rs::{Element, QueueError};

/// Minimal handle that delivers envelopes into an actor's mailbox.
///
/// Unlike [`ActorRef`](ActorRef) this type operates on the
/// raw mailbox types and is primarily used by runtime internals. It lives in
/// the `api` layer so that other public types (such as [`ActorRef`]) can hold
/// it without depending on `internal` modules.
pub struct PriorityActorRef<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  sender: R::Producer<PriorityEnvelope<M>>,
}

unsafe impl<M, R> Send for PriorityActorRef<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone + RuntimeBound,
  R::Signal: Clone + RuntimeBound,
{
}

unsafe impl<M, R> Sync for PriorityActorRef<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone + RuntimeBound,
  R::Signal: Clone + RuntimeBound,
{
}

impl<M, R> Clone for PriorityActorRef<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      sender: self.sender.clone(),
    }
  }
}

impl<M, R> PriorityActorRef<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  /// Wraps a mailbox producer handle.
  #[must_use]
  pub fn new(sender: R::Producer<PriorityEnvelope<M>>) -> Self {
    Self { sender }
  }

  /// Sends a message with the specified priority to the mailbox.
  pub fn try_send_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(PriorityEnvelope::new(message, priority))
  }

  /// Sends a control-channel message with the specified priority.
  pub fn try_send_control_with_priority(
    &self,
    message: M,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(PriorityEnvelope::control(message, priority))
  }

  /// Sends a pre-built priority envelope.
  pub fn try_send_envelope(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(envelope)
  }

  /// Returns the raw producer handle kept by the reference.
  #[must_use]
  pub fn sender(&self) -> &R::Producer<PriorityEnvelope<M>> {
    &self.sender
  }
}

impl<R> PriorityActorRef<SystemMessage, R>
where
  R: MailboxFactory,
  R::Producer<PriorityEnvelope<SystemMessage>>: Clone,
{
  /// Sends a system message via the reference.
  pub fn try_send_system(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<SystemMessage>>> {
    self.sender.try_send(PriorityEnvelope::from_system(message))
  }
}
