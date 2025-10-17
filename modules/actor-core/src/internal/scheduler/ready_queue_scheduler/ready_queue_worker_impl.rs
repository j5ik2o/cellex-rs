use futures::future::LocalBoxFuture;
use spin::Mutex;

use crate::api::mailbox::messages::PriorityEnvelope;
use crate::internal::guardian::GuardianStrategy;
use crate::{MailboxRuntime, ReadyQueueWorker};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

use super::ready_queue_context::ReadyQueueContext;
pub(crate) struct ReadyQueueWorkerImpl<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  context: ArcShared<Mutex<ReadyQueueContext<M, R, Strat>>>,
}

impl<M, R, Strat> ReadyQueueWorkerImpl<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  pub(crate) fn new(context: ArcShared<Mutex<ReadyQueueContext<M, R, Strat>>>) -> Self {
    Self { context }
  }
}

impl<M, R, Strat> ReadyQueueWorker<M, R> for ReadyQueueWorkerImpl<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
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
