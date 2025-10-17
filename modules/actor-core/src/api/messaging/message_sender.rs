use crate::api::mailbox::mailbox_concurrency::MailboxConcurrency;
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::mailbox::thread_safe::ThreadSafe;
use crate::api::messaging::DynMessage;
use crate::api::messaging::MessageEnvelope;
use crate::internal::message::internal_message_sender::InternalMessageSender;
use cellex_utils_core_rs::{Element, QueueError};
use core::marker::PhantomData;

/// Type-safe dispatcher. Wraps the internal dispatcher and automatically envelopes user messages.
#[derive(Clone)]
pub struct MessageSender<M, C: MailboxConcurrency = ThreadSafe>
where
  M: Element, {
  inner: InternalMessageSender<C>,
  _marker: PhantomData<fn(M)>,
}

impl<M, C> core::fmt::Debug for MessageSender<M, C>
where
  M: Element,
  C: MailboxConcurrency,
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_tuple("MessageSender").finish()
  }
}

impl<M, C> MessageSender<M, C>
where
  M: Element,
  C: MailboxConcurrency,
{
  /// Creates a typed `MessageSender` from an internal sender (internal API).
  ///
  /// # Arguments
  /// * `inner` - Internal message sender
  pub(crate) fn new(inner: InternalMessageSender<C>) -> Self {
    Self {
      inner,
      _marker: PhantomData,
    }
  }

  /// Creates a typed `MessageSender` from an internal sender.
  ///
  /// # Arguments
  /// * `inner` - Internal message sender
  pub fn from_internal(inner: InternalMessageSender<C>) -> Self {
    Self::new(inner)
  }

  /// Dispatches a user message.
  ///
  /// # Arguments
  /// * `message` - User message to send
  ///
  /// # Returns
  /// `Ok(())` on success, queue error on failure
  pub fn dispatch_user(&self, message: M) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.dispatch_envelope(MessageEnvelope::user(message))
  }

  /// Dispatches a message envelope.
  ///
  /// # Arguments
  /// * `envelope` - Message envelope to send
  ///
  /// # Returns
  /// `Ok(())` on success, queue error on failure
  pub fn dispatch_envelope(
    &self,
    envelope: MessageEnvelope<M>,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let dyn_message = DynMessage::new(envelope);
    self.inner.send_default(dyn_message)
  }

  /// Dispatches a message envelope with the specified priority.
  ///
  /// # Arguments
  /// * `envelope` - Message envelope to send
  /// * `priority` - Message priority
  ///
  /// # Returns
  /// `Ok(())` on success, queue error on failure
  pub fn dispatch_with_priority(
    &self,
    envelope: MessageEnvelope<M>,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let dyn_message = DynMessage::new(envelope);
    self.inner.send_with_priority(dyn_message, priority)
  }

  /// Gets a clone of the internal sender.
  ///
  /// # Returns
  /// Clone of the internal message sender
  pub fn internal(&self) -> InternalMessageSender<C> {
    self.inner.clone()
  }

  /// Converts to the internal sender, transferring ownership.
  ///
  /// # Returns
  /// Internal message sender
  pub fn into_internal(self) -> InternalMessageSender<C> {
    self.inner
  }
}
