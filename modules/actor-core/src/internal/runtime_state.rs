use cellex_utils_core_rs::sync::{ArcShared, Shared};

use crate::api::mailbox::PriorityEnvelope;
use crate::internal::mailbox::traits::MailboxRuntime;
use crate::internal::scheduler::SchedulerBuilder;
use crate::DynMessage;

/// Internal state container for `GenericActorRuntime`.
///
/// This structure holds the mailbox runtime and scheduler builder configuration
/// used by the actor runtime implementation.
#[derive(Clone)]
pub(crate) struct GenericActorRuntimeState<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  mailbox_runtime: ArcShared<R>,
  scheduler_builder: ArcShared<SchedulerBuilder<DynMessage, R>>,
}

impl<R> GenericActorRuntimeState<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new runtime state with the given mailbox runtime.
  ///
  /// Initializes with a default ready-queue scheduler builder.
  #[must_use]
  pub(crate) fn new(actor_runtime: R) -> Self {
    Self {
      mailbox_runtime: ArcShared::new(actor_runtime),
      scheduler_builder: ArcShared::new(SchedulerBuilder::<DynMessage, R>::ready_queue()),
    }
  }

  /// Returns a reference to the mailbox runtime.
  #[must_use]
  pub(crate) fn mailbox_runtime(&self) -> &R {
    &self.mailbox_runtime
  }

  /// Returns a shared handle to the mailbox runtime.
  #[must_use]
  pub(crate) fn mailbox_runtime_shared(&self) -> ArcShared<R> {
    self.mailbox_runtime.clone()
  }

  /// Consumes this state and returns the mailbox runtime.
  ///
  /// If the shared handle has other references, clones the runtime.
  #[must_use]
  pub(crate) fn into_mailbox_runtime(self) -> R {
    self
      .mailbox_runtime
      .try_unwrap()
      .unwrap_or_else(|shared| (*shared).clone())
  }

  /// Returns a shared handle to the scheduler builder.
  #[must_use]
  pub(crate) fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, R>> {
    self.scheduler_builder.clone()
  }

  /// Sets the scheduler builder for this runtime state.
  pub(crate) fn set_scheduler_builder(&mut self, builder: ArcShared<SchedulerBuilder<DynMessage, R>>) {
    self.scheduler_builder = builder;
  }
}
