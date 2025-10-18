use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::receive_timeout::{ReceiveTimeoutSchedulerFactoryProviderBound, ReceiveTimeoutSchedulerFactoryShared};

/// Trait representing a runtime-specific provider for receive-timeout scheduler factories.
pub trait ReceiveTimeoutSchedulerFactoryProvider<MF>: ReceiveTimeoutSchedulerFactoryProviderBound
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<DynMessage>>: Clone, {
  /// Builds a shared factory bound to the use cellex_actor_core_rs::api::mailbox::MailboxRuntime; for the given actor runtime.
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<DynMessage, MF>;
}
