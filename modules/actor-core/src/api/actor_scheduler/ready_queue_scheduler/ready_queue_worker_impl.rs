use cellex_utils_core_rs::{sync::ArcShared, v2::collections::queue::backend::QueueError};
use futures::future::LocalBoxFuture;
use spin::Mutex;

use super::ready_queue_context::ReadyQueueContext;
use crate::{
  api::{actor_scheduler::ready_queue_scheduler::ReadyQueueWorker, guardian::GuardianStrategy},
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::AnyMessage,
  },
};

pub(crate) struct ReadyQueueWorkerImpl<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  context: ArcShared<Mutex<ReadyQueueContext<MF, Strat>>>,
}

impl<MF, Strat> ReadyQueueWorkerImpl<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  pub(crate) const fn new(context: ArcShared<Mutex<ReadyQueueContext<MF, Strat>>>) -> Self {
    Self { context }
  }
}

impl<MF, Strat> ReadyQueueWorker<MF> for ReadyQueueWorkerImpl<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<AnyMessage>>> {
    let mut ctx = self.context.lock();
    ctx.process_ready_once()
  }

  fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>> {
    let ctx = self.context.lock();
    ctx.wait_for_any_signal_future()
  }
}
