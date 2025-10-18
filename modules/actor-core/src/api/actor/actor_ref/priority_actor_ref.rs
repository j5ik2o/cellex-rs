use cellex_utils_core_rs::{Element, QueueError};

use crate::{
  api::mailbox::{MailboxFactory, MailboxProducer, PriorityEnvelope, SystemMessage},
  RuntimeBound,
};

/// Minimal handle that delivers envelopes into an actor's mailbox.
///
/// Unlike [`ActorRef`](ActorRef) this type operates on the
/// raw mailbox types and is primarily used by runtime internals. It lives in
/// the `api` layer so that other public types (such as [`ActorRef`]) can hold
/// it without depending on `internal` modules.
pub struct PriorityActorRef<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  sender: MF::Producer<PriorityEnvelope<M>>,
}

unsafe impl<M, MF> Send for PriorityActorRef<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone + RuntimeBound,
  MF::Signal: Clone + RuntimeBound,
{
}

unsafe impl<M, MF> Sync for PriorityActorRef<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone + RuntimeBound,
  MF::Signal: Clone + RuntimeBound,
{
}

impl<M, MF> Clone for PriorityActorRef<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<M>>: Clone,
{
  fn clone(&self) -> Self {
    Self { sender: self.sender.clone() }
  }
}

impl<M, MF> PriorityActorRef<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<M>>: Clone,
{
  /// Wraps a mailbox producer handle.
  #[must_use]
  pub fn new(sender: MF::Producer<PriorityEnvelope<M>>) -> Self {
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
  pub fn sender(&self) -> &MF::Producer<PriorityEnvelope<M>> {
    &self.sender
  }
}

impl<MF> PriorityActorRef<SystemMessage, MF>
where
  MF: MailboxFactory,
  MF::Producer<PriorityEnvelope<SystemMessage>>: Clone,
{
  /// Sends a system message via the reference.
  pub fn try_send_system(&self, message: SystemMessage) -> Result<(), QueueError<PriorityEnvelope<SystemMessage>>> {
    self.sender.try_send(PriorityEnvelope::from_system(message))
  }
}
