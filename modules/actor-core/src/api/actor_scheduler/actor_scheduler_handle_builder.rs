use alloc::boxed::Box;

use cellex_utils_core_rs::{
  sync::{ArcShared, Shared},
  SharedBound,
};

#[cfg(any(test, feature = "test-support"))]
use crate::api::actor_scheduler::immediate_scheduler::ImmediateScheduler;
use crate::api::{
  actor_scheduler::{
    actor_scheduler_handle::ActorSchedulerHandle, ready_queue_scheduler::ReadyQueueScheduler,
    ActorSchedulerHandleFactoryFn,
  },
  extensions::Extensions,
  guardian::GuardianStrategy,
  mailbox::{MailboxFactory, PriorityEnvelope},
  messaging::DynMessage,
};

/// Factory wrapper used to construct scheduler instances with consistent runtime configuration.
#[derive(Clone)]
pub struct ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone, {
  factory: ArcShared<ActorSchedulerHandleFactoryFn<MF>>,
}

impl<MF> ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
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
    F: Fn(MF, Extensions) -> ActorSchedulerHandle<MF> + SharedBound + 'static, {
    let shared = ArcShared::new(factory);
    Self { factory: shared.into_dyn(|inner| inner as &ActorSchedulerHandleFactoryFn<MF>) }
  }

  /// Builds a scheduler using the stored factory and provided runtime components.
  pub fn build(&self, mailbox_factory: MF, extensions: Extensions) -> ActorSchedulerHandle<MF> {
    self.factory.with_ref(|factory| (factory)(mailbox_factory, extensions))
  }
}
impl<MF> ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone,
{
  /// Returns a builder configured to create ready-queue-based schedulers.
  pub fn ready_queue() -> Self {
    Self::new(|mailbox_factory, extensions| Box::new(ReadyQueueScheduler::new(mailbox_factory, extensions)))
  }

  #[allow(dead_code)]
  /// Returns a builder that wires a custom guardian strategy into the ready-queue scheduler.
  pub fn with_strategy<Strat>(self, strategy: Strat) -> Self
  where
    Strat: GuardianStrategy<MF> + Clone + Send + Sync, {
    let _ = self;
    Self::new(move |mailbox_factory, extensions| {
      Box::new(ReadyQueueScheduler::with_strategy(mailbox_factory, strategy.clone(), extensions))
    })
  }
}
