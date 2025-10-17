use alloc::boxed::Box;

use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::scheduler::noop_receive_timeout_scheduler::NoopReceiveTimeoutScheduler;
use crate::shared::map_system::MapSystemShared;
use crate::shared::receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory};
use cellex_utils_core_rs::Element;

/// Factory that returns [`NoopReceiveTimeoutScheduler`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutSchedulerFactory;

impl<M, R> ReceiveTimeoutSchedulerFactory<M, R> for NoopReceiveTimeoutSchedulerFactory
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
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
