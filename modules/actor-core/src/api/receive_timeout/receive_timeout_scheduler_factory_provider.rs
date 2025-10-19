use crate::api::{
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  messaging::AnyMessage,
  receive_timeout::{ReceiveTimeoutSchedulerFactoryProviderBound, ReceiveTimeoutSchedulerFactoryShared},
};

/// Trait representing a runtime-specific provider for receive-timeout scheduler factories.
pub trait ReceiveTimeoutSchedulerFactoryProvider<MF>: ReceiveTimeoutSchedulerFactoryProviderBound
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
  /// Builds a shared factory bound to the use cellex_actor_core_rs::api::mailbox::MailboxRuntime;
  /// for the given actor runtime.
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>;
}
