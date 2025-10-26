use alloc::boxed::Box;
use core::{convert::Infallible, future::Future, pin::Pin};

use cellex_utils_core_rs::{collections::Element, sync::ArcShared, v2::collections::queue::backend::QueueError};

use crate::{
  api::{
    actor::{RootContext, ShutdownToken},
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    actor_scheduler::ready_queue_scheduler::ReadyQueueWorker,
    guardian::GuardianStrategy,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Common actor system interface.
///
/// Exposes the minimum set of operations that runtime drivers and test helpers rely on.
pub trait ActorSystem<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>, {
  /// Returns the shutdown token shared across the entire system.
  fn shutdown_token(&self) -> ShutdownToken;

  /// Borrows the root context for spawning and coordination.
  fn root_context(&mut self) -> RootContext<'_, U, AR, Strat>;

  /// Returns the handle for ReadyQueue workers when available.
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>>;

  /// Indicates whether the runtime supports ReadyQueue-based scheduling.
  fn supports_ready_queue(&self) -> bool;

  /// Drains the ready queue synchronously until it becomes empty.
  fn run_until_idle(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>;

  /// Continues dispatching messages until the supplied predicate returns `false`.
  fn run_until<'a, F>(
    &'a mut self,
    should_continue: F,
  ) -> Pin<Box<dyn Future<Output = Result<(), QueueError<PriorityEnvelope<AnyMessage>>>> + 'a>>
  where
    F: FnMut() -> bool + 'a;

  /// Continues dispatching messages until explicitly stopped.
  fn run_forever(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>> + '_>>;

  /// Processes the next available message exactly once.
  fn dispatch_next(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = Result<(), QueueError<PriorityEnvelope<AnyMessage>>>> + '_>>;
}
