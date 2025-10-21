use alloc::boxed::Box;

use cellex_utils_core_rs::Element;

use crate::{
  api::{
    mailbox::MailboxFactory,
    receive_timeout::{
      noop_receive_timeout_scheduler::NoopReceiveTimeoutScheduler, ReceiveTimeoutScheduler,
      ReceiveTimeoutSchedulerFactory,
    },
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::MapSystemShared},
};

/// Factory that returns [`NoopReceiveTimeoutScheduler`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutSchedulerFactory;

impl<M, MF> ReceiveTimeoutSchedulerFactory<M, MF> for NoopReceiveTimeoutSchedulerFactory
where
  M: Element + 'static,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<M>>: Clone,
{
  fn create(
    &self,
    _sender: MF::Producer<PriorityEnvelope<M>>,
    _map_system: MapSystemShared<M>,
  ) -> Box<dyn ReceiveTimeoutScheduler> {
    Box::new(NoopReceiveTimeoutScheduler)
  }
}
