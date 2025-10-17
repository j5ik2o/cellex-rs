use crate::api::mailbox::mailbox_runtime::MailboxRuntime;
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::internal::scheduler::noop_receive_timeout_scheduler_factory::NoopReceiveTimeoutSchedulerFactory;
use crate::shared::receive_timeout::ReceiveTimeoutDriver;
use crate::shared::receive_timeout::ReceiveTimeoutFactoryShared;

/// Driver that always provides [`NoopReceiveTimeoutSchedulerFactory`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutDriver;

impl<R> ReceiveTimeoutDriver<R> for NoopReceiveTimeoutDriver
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, R> {
    ReceiveTimeoutFactoryShared::new(NoopReceiveTimeoutSchedulerFactory)
  }
}
