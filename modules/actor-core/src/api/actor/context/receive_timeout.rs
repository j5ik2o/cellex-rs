use super::Context;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::{MailboxRuntime, MetadataStorageMode};
use cellex_utils_core_rs::Element;
use core::time::Duration;

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, crate::api::mailbox::PriorityEnvelope<crate::DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Determines if receive timeout is supported.
  #[must_use]
  pub fn has_receive_timeout_support(&self) -> bool {
    self.inner.has_receive_timeout_scheduler()
  }

  /// Sets the receive timeout.
  pub fn set_receive_timeout(&mut self, duration: Duration) -> bool {
    self.inner.set_receive_timeout(duration)
  }

  /// Cancels the receive timeout.
  pub fn cancel_receive_timeout(&mut self) -> bool {
    self.inner.cancel_receive_timeout()
  }
}
