use crate::api::mailbox::PriorityEnvelope;
use crate::{
  ActorRuntime, ActorSystem, AlwaysRestart, DynMessage, MailboxOf, MailboxQueueOf, MailboxSignalOf, ReadyQueueWorker,
  ShutdownToken,
};
use cellex_utils_core_rs::{ArcShared, Element, QueueError};
use core::convert::Infallible;
use core::marker::PhantomData;
use core::num::NonZeroUsize;

/// Execution runner for the actor system.
///
/// Wraps `ActorSystem` and provides an interface for execution on an asynchronous runtime.
pub struct ActorSystemRunner<U, R, Strat = AlwaysRestart>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  Strat: crate::GuardianStrategy<DynMessage, MailboxOf<R>>, {
  pub(crate) system: ActorSystem<U, R, Strat>,
  pub(crate) ready_queue_worker_count: NonZeroUsize,
  pub(crate) _marker: PhantomData<U>,
}

impl<U, R, Strat> ActorSystemRunner<U, R, Strat>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  Strat: crate::GuardianStrategy<DynMessage, MailboxOf<R>>,
{
  /// Gets the number of ReadyQueue workers to spawn when driving the system.
  #[must_use]
  pub fn ready_queue_worker_count(&self) -> NonZeroUsize {
    self.ready_queue_worker_count
  }

  /// Updates the ReadyQueue worker count in place.
  pub fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize) {
    self.ready_queue_worker_count = worker_count;
  }

  /// Returns a new runner with the specified ReadyQueue worker count.
  #[must_use]
  pub fn with_ready_queue_worker_count(mut self, worker_count: NonZeroUsize) -> Self {
    self.set_ready_queue_worker_count(worker_count);
    self
  }

  /// Returns a ReadyQueue worker handle if supported by the underlying scheduler.
  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<DynMessage, MailboxOf<R>>>> {
    self.system.ready_queue_worker()
  }

  /// Indicates whether the scheduler supports ReadyQueue-based execution.
  #[must_use]
  pub fn supports_ready_queue(&self) -> bool {
    self.system.supports_ready_queue()
  }

  /// Gets the shutdown token.
  ///
  /// # Returns
  /// Clone of the shutdown token
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.system.shutdown.clone()
  }

  /// Executes message dispatching permanently.
  ///
  /// This function does not terminate normally. Returns only on error.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  pub async fn run_forever(mut self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.system.run_forever().await
  }

  /// Executes the runner as a Future.
  ///
  /// Alias for `run_forever`. Provides a name suitable for execution on an asynchronous runtime.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  pub async fn into_future(self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.run_forever().await
  }

  /// Extracts the internal actor system from the runner.
  ///
  /// # Returns
  /// Internal actor system
  pub fn into_inner(self) -> ActorSystem<U, R, Strat> {
    self.system
  }
}
