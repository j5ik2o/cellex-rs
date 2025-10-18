use alloc::boxed::Box;

use cellex_utils_core_rs::{
  sync::{ArcShared, Shared},
  Element, SharedBound,
};

#[cfg(any(test, feature = "test-support"))]
use crate::api::actor_scheduler::immediate_scheduler::ImmediateScheduler;
use crate::{
  api::{
    actor_scheduler::{
      actor_scheduler_handle::ActorSchedulerHandle, ready_queue_scheduler::ReadyQueueScheduler,
      ActorSchedulerHandleFactoryFn,
    },
    extensions::Extensions,
    mailbox::{MailboxFactory, PriorityEnvelope},
  },
  internal::guardian::GuardianStrategy,
};

/// Factory wrapper used to construct scheduler instances with consistent runtime configuration.
#[derive(Clone)]
pub struct ActorSchedulerHandleBuilder<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  factory: ArcShared<ActorSchedulerHandleFactoryFn<M, MF>>,
}

impl<M, MF> ActorSchedulerHandleBuilder<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
{
  #[cfg(any(test, feature = "test-support"))]
  #[allow(dead_code)]
  #[must_use]
  /// Creates a builder that produces the immediate scheduler used in tests.
  pub fn immediate() -> Self {
    Self::new(|mailbox_factory, extensions| Box::new(ImmediateScheduler::new(mailbox_factory, extensions)))
  }

  /// Creates a builder from a factory closure producing scheduler handles.
  pub fn new<F>(factory: F) -> Self
  where
    F: Fn(MF, Extensions) -> ActorSchedulerHandle<M, MF> + SharedBound + 'static, {
    let shared = ArcShared::new(factory);
    Self { factory: shared.into_dyn(|inner| inner as &ActorSchedulerHandleFactoryFn<M, MF>) }
  }

  /// Builds a scheduler using the stored factory and provided runtime components.
  pub fn build(&self, mailbox_factory: MF, extensions: Extensions) -> ActorSchedulerHandle<M, MF> {
    self.factory.with_ref(|factory| (factory)(mailbox_factory, extensions))
  }
}
impl<M, MF> ActorSchedulerHandleBuilder<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
{
  /// Returns a builder configured to create ready-queue-based schedulers.
  pub fn ready_queue() -> Self {
    Self::new(|mailbox_factory, extensions| Box::new(ReadyQueueScheduler::new(mailbox_factory, extensions)))
  }

  #[allow(dead_code)]
  /// Returns a builder that wires a custom guardian strategy into the ready-queue scheduler.
  pub fn with_strategy<Strat>(self, strategy: Strat) -> Self
  where
    Strat: GuardianStrategy<M, MF> + Clone + Send + Sync, {
    let _ = self;
    Self::new(move |mailbox_factory, extensions| {
      Box::new(ReadyQueueScheduler::with_strategy(mailbox_factory, strategy.clone(), extensions))
    })
  }
}
