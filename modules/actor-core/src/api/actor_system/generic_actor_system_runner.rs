use alloc::boxed::Box;
use core::{convert::Infallible, future::Future, marker::PhantomData, num::NonZeroUsize, pin::Pin};

use cellex_utils_core_rs::{collections::queue::QueueError, ArcShared, Element};

use crate::{
  api::{
    actor::ShutdownToken,
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    actor_scheduler::ready_queue_scheduler::ReadyQueueWorker,
    actor_system::{ActorSystemRunner, GenericActorSystem},
    guardian::AlwaysRestart,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Execution runner for the actor system.
///
/// Wraps `GenericActorSystem` and provides an interface for execution on an asynchronous runtime.
pub struct GenericActorSystemRunner<U, AR, Strat = AlwaysRestart>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>, {
  pub(crate) system:                   GenericActorSystem<U, AR, Strat>,
  pub(crate) ready_queue_worker_count: NonZeroUsize,
  pub(crate) _marker:                  PhantomData<U>,
}

impl<U, AR, Strat> GenericActorSystemRunner<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>,
{
  /// Gets the number of ReadyQueue workers to spawn when driving the system.
  #[must_use]
  pub const fn ready_queue_worker_count(&self) -> NonZeroUsize {
    self.ready_queue_worker_count
  }

  /// Updates the ReadyQueue worker count in place.
  pub const fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize) {
    self.ready_queue_worker_count = worker_count;
  }

  /// Returns a new runner with the specified ReadyQueue worker count.
  #[must_use]
  pub const fn with_ready_queue_worker_count(mut self, worker_count: NonZeroUsize) -> Self {
    self.set_ready_queue_worker_count(worker_count);
    self
  }

  /// Returns a ReadyQueue worker handle if supported by the underlying scheduler.
  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>> {
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
  #[must_use]
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.system.shutdown.clone()
  }

  /// Executes message dispatching permanently.
  ///
  /// This function does not terminate normally. Returns only on error.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue processing fails.
  pub async fn run_forever(mut self) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.system.run_forever().await
  }

  /// Executes the runner as a Future.
  ///
  /// Alias for `run_forever`. Provides a name suitable for execution on an asynchronous runtime.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue processing fails.
  pub async fn into_future(self) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.run_forever().await
  }

  /// Extracts the internal actor system from the runner.
  ///
  /// # Returns
  /// Internal actor system
  #[must_use]
  pub fn into_inner(self) -> GenericActorSystem<U, AR, Strat> {
    self.system
  }
}

impl<U, AR, Strat> ActorSystemRunner<U, AR, Strat> for GenericActorSystemRunner<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>,
{
  type System = GenericActorSystem<U, AR, Strat>;

  fn ready_queue_worker_count(&self) -> NonZeroUsize {
    self.ready_queue_worker_count
  }

  fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize) {
    self.ready_queue_worker_count = worker_count;
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>> {
    self.system.ready_queue_worker()
  }

  fn supports_ready_queue(&self) -> bool {
    self.system.supports_ready_queue()
  }

  fn shutdown_token(&self) -> ShutdownToken {
    self.system.shutdown_token()
  }

  fn run_forever(
    self,
  ) -> Pin<Box<dyn Future<Output = Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>> + 'static>> {
    Box::pin(async move {
      let mut system = self.system;
      system.run_forever().await
    })
  }

  fn into_inner(self) -> Self::System {
    self.system
  }
}
