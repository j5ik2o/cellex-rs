use crate::api::{
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  messaging::AnyMessage,
  receive_timeout::{
    noop_receive_timeout_scheduler_factory::NoopReceiveTimeoutSchedulerFactory, ReceiveTimeoutSchedulerFactoryProvider,
    ReceiveTimeoutSchedulerFactoryShared,
  },
};

/// Driver that always provides [`NoopReceiveTimeoutSchedulerFactory`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutSchedulerFactoryProvider;

impl<MF> ReceiveTimeoutSchedulerFactoryProvider<MF> for NoopReceiveTimeoutSchedulerFactoryProvider
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<AnyMessage>>: Clone,
{
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF> {
    ReceiveTimeoutSchedulerFactoryShared::new(NoopReceiveTimeoutSchedulerFactory)
  }
}
