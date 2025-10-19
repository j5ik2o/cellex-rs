#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use core::marker::PhantomData;

use cellex_utils_core_rs::{ArcShared, QueueError, DEFAULT_PRIORITY};

use crate::{
  api::{
    actor::actor_ref::PriorityActorRef,
    mailbox::{messages::PriorityEnvelope, MailboxConcurrency, MailboxFactory, ThreadSafe},
    messaging::AnyMessage,
  },
  RuntimeBound,
};

#[cfg(target_has_atomic = "ptr")]
type DropHookFn = dyn Fn() + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type DropHookFn = dyn Fn();

#[cfg(target_has_atomic = "ptr")]
type SendFn = dyn Fn(AnyMessage, i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type SendFn = dyn Fn(AnyMessage, i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>;

/// Internal dispatcher that abstracts the sending destination. Used for ask responses and similar
/// purposes.
#[derive(Clone)]
pub struct InternalMessageSender<C: MailboxConcurrency = ThreadSafe> {
  inner:     ArcShared<SendFn>,
  drop_hook: Option<ArcShared<DropHookFn>>,
  _marker:   PhantomData<C>,
}

impl<C> core::fmt::Debug for InternalMessageSender<C>
where
  C: MailboxConcurrency,
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("MessageSender(..)")
  }
}

impl<C> InternalMessageSender<C>
where
  C: MailboxConcurrency,
{
  /// Creates a new `InternalMessageSender` with the specified send function.
  ///
  /// # Arguments
  /// * `inner` - Function that executes message sending
  pub fn new(inner: ArcShared<SendFn>) -> Self {
    Self { inner, drop_hook: None, _marker: PhantomData }
  }

  /// Creates an `InternalMessageSender` with a drop hook (internal API).
  ///
  /// # Arguments
  /// * `inner` - Function that executes message sending
  /// * `drop_hook` - Hook function executed on drop
  pub(crate) fn with_drop_hook(inner: ArcShared<SendFn>, drop_hook: ArcShared<DropHookFn>) -> Self {
    Self { inner, drop_hook: Some(drop_hook), _marker: PhantomData }
  }

  /// Sends a message with default priority.
  ///
  /// # Arguments
  /// * `message` - Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, queue error on failure
  pub fn send_default(&self, message: AnyMessage) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.send_with_priority(message, DEFAULT_PRIORITY)
  }

  /// Sends a message with the specified priority.
  ///
  /// # Arguments
  /// * `message` - Message to send
  /// * `priority` - Message priority
  ///
  /// # Returns
  /// `Ok(())` on success, queue error on failure
  pub fn send_with_priority(
    &self,
    message: AnyMessage,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    (self.inner)(message, priority)
  }
}

impl<C> Drop for InternalMessageSender<C>
where
  C: MailboxConcurrency,
{
  fn drop(&mut self) {
    if let Some(hook) = &self.drop_hook {
      hook();
    }
  }
}

impl InternalMessageSender {
  /// Thread-safe helper retained for existing call sites.
  #[allow(dead_code)]
  pub(crate) fn from_internal_ref<MF>(actor_ref: PriorityActorRef<AnyMessage, MF>) -> Self
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<AnyMessage>>: Clone + RuntimeBound + 'static,
    MF::Signal: Clone + RuntimeBound + 'static, {
    let sender = actor_ref.clone();
    Self::new(ArcShared::from_arc_for_testing_dont_use_production(Arc::new(move |message, priority| {
      sender.try_send_with_priority(message, priority)
    })))
  }
}
