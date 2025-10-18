use alloc::boxed::Box;

use crate::api::actor_system::map_system::MapSystemShared;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory};
use crate::internal::scheduler::noop_receive_timeout_scheduler::NoopReceiveTimeoutScheduler;
use cellex_utils_core_rs::Element;

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
