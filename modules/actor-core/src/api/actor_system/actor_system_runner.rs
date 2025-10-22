use alloc::boxed::Box;
use core::{convert::Infallible, future::Future, num::NonZeroUsize, pin::Pin};

use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};

use super::ActorSystem;
use crate::{
  api::{
    actor::ShutdownToken,
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    actor_scheduler::ready_queue_scheduler::ReadyQueueWorker,
    guardian::GuardianStrategy,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Abstracts the execution surface for actor systems when driven on an async runtime.
pub trait ActorSystemRunner<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>,
  Self: Sized, {
  /// Actor system instance consumed or borrowed by the runner.
  type System: ActorSystem<U, AR, Strat>;

  /// Gets the number of ReadyQueue worker tasks to spawn.
  fn ready_queue_worker_count(&self) -> NonZeroUsize;

  /// Updates the ReadyQueue worker task count.
  fn set_ready_queue_worker_count(&mut self, worker_count: NonZeroUsize);

  /// Returns a new runner with the specified ReadyQueue worker task count.
  fn with_ready_queue_worker_count(mut self, worker_count: NonZeroUsize) -> Self {
    self.set_ready_queue_worker_count(worker_count);
    self
  }

  /// Returns a ReadyQueue worker handle when available.
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>>;

  /// Indicates whether the underlying scheduler supports ReadyQueue execution.
  fn supports_ready_queue(&self) -> bool;

  /// Gets the shutdown token propagated across the actor system.
  fn shutdown_token(&self) -> ShutdownToken;

  /// Drives message dispatching until termination.
  fn run_forever(
    self,
  ) -> Pin<Box<dyn Future<Output = Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>> + 'static>>;

  /// Executes the runner as a future.
  fn into_future(
    self,
  ) -> Pin<Box<dyn Future<Output = Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>> + 'static>> {
    self.run_forever()
  }

  /// Extracts the underlying actor system instance.
  fn into_inner(self) -> Self::System;
}
