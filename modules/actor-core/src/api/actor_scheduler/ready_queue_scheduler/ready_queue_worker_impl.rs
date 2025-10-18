use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};
use futures::future::LocalBoxFuture;
use spin::Mutex;

use super::ready_queue_context::ReadyQueueContext;
use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueWorker,
    mailbox::{MailboxFactory, PriorityEnvelope},
  },
  internal::guardian::GuardianStrategy,
};

pub(crate) struct ReadyQueueWorkerImpl<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>, {
  context: ArcShared<Mutex<ReadyQueueContext<M, MF, Strat>>>,
}

impl<M, MF, Strat> ReadyQueueWorkerImpl<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  pub(crate) fn new(context: ArcShared<Mutex<ReadyQueueContext<M, MF, Strat>>>) -> Self {
    Self { context }
  }
}

impl<M, MF, Strat> ReadyQueueWorker<M, MF> for ReadyQueueWorkerImpl<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<M>>> {
    let mut ctx = self.context.lock();
    ctx.process_ready_once()
  }

  fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>> {
    let ctx = self.context.lock();
    ctx.wait_for_any_signal_future()
  }
}
