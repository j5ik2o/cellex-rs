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

impl<M, R> ReceiveTimeoutSchedulerFactory<M, R> for NoopReceiveTimeoutSchedulerFactory
where
  M: Element + 'static,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  fn create(
    &self,
    _sender: R::Producer<PriorityEnvelope<M>>,
    _map_system: MapSystemShared<M>,
  ) -> Box<dyn ReceiveTimeoutScheduler> {
    Box::new(NoopReceiveTimeoutScheduler)
  }
}
