use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::MailboxProducer;
use crate::{MailboxRuntime, RuntimeBound};
use cellex_utils_core_rs::{Element, QueueError};

/// Actor reference. Wraps QueueMailboxProducer and provides message sending API.
pub struct InternalActorRef<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  sender: R::Producer<PriorityEnvelope<M>>,
}

unsafe impl<M, R> Send for InternalActorRef<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone + RuntimeBound,
  R::Signal: Clone + RuntimeBound,
{
}

unsafe impl<M, R> Sync for InternalActorRef<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone + RuntimeBound,
  R::Signal: Clone + RuntimeBound,
{
}

impl<M, R> Clone for InternalActorRef<M, R>
where
  M: Element,
  R: MailboxRuntime,
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

impl<M, R> InternalActorRef<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  /// Creates a new internal actor reference backed by the provided producer.
  pub fn new(sender: R::Producer<PriorityEnvelope<M>>) -> Self {
    Self { sender }
  }

  /// Sends a message with the specified priority to the underlying mailbox.
  pub fn try_send_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(PriorityEnvelope::new(message, priority))
  }

  #[allow(dead_code)]
  /// Sends a control message with the specified priority to the underlying mailbox.
  pub fn try_send_control_with_priority(
    &self,
    message: M,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(PriorityEnvelope::control(message, priority))
  }

  #[allow(dead_code)]
  /// Sends a pre-built priority envelope to the underlying mailbox.
  pub fn try_send_envelope(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(envelope)
  }

  /// Returns the raw producer handle used by the actor reference.
  pub fn sender(&self) -> &R::Producer<PriorityEnvelope<M>> {
    &self.sender
  }
}

impl<R> InternalActorRef<SystemMessage, R>
where
  R: MailboxRuntime,
  R::Producer<PriorityEnvelope<SystemMessage>>: Clone,
{
  #[allow(dead_code)]
  /// Sends a system message through the actor reference.
  pub fn try_send_system(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<SystemMessage>>> {
    self.sender.try_send(PriorityEnvelope::from_system(message))
  }
}
