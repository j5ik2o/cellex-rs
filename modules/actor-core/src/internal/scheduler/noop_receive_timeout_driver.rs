use crate::{
  api::{
    mailbox::{MailboxFactory, PriorityEnvelope},
    messaging::DynMessage,
    receive_timeout::{ReceiveTimeoutSchedulerFactoryProvider, ReceiveTimeoutSchedulerFactoryShared},
  },
  internal::scheduler::noop_receive_timeout_scheduler_factory::NoopReceiveTimeoutSchedulerFactory,
};

/// Driver that always provides [`NoopReceiveTimeoutSchedulerFactory`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutSchedulerFactoryProvider;

impl<MF> ReceiveTimeoutSchedulerFactoryProvider<MF> for NoopReceiveTimeoutSchedulerFactoryProvider
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<DynMessage, MF> {
    ReceiveTimeoutSchedulerFactoryShared::new(NoopReceiveTimeoutSchedulerFactory)
  }
}
