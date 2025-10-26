use cellex_utils_core_rs::SharedBound;

use crate::{
  api::{mailbox::MailboxFactory, receive_timeout::ReceiveTimeoutSchedulerFactoryShared},
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Trait representing a runtime-specific provider for receive-timeout scheduler factories.
pub trait ReceiveTimeoutSchedulerFactoryProvider<MF>: SharedBound
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
  /// Builds a shared factory bound to the mailbox factory for the given actor runtime.
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>;
}
