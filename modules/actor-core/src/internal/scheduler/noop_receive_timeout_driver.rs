use crate::{
  DynMessage, MailboxRuntime, NoopReceiveTimeoutSchedulerFactory, PriorityEnvelope, ReceiveTimeoutDriver,
  ReceiveTimeoutFactoryShared,
};

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
