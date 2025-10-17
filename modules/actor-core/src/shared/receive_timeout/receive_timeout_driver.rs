use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;

use super::receive_timeout_driver_bound::ReceiveTimeoutDriverBound;
use super::receive_timeout_factory_shared::ReceiveTimeoutFactoryShared;

/// Trait representing a runtime-specific provider for receive-timeout scheduler factories.
pub trait ReceiveTimeoutDriver<R>: ReceiveTimeoutDriverBound
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone, {
  /// Builds a shared factory bound to the mailbox runtime for the given actor runtime.
  fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, R>;
}
