use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::receive_timeout::ReceiveTimeoutSchedulerFactoryProvider;
use crate::api::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use crate::internal::scheduler::noop_receive_timeout_scheduler_factory::NoopReceiveTimeoutSchedulerFactory;

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
