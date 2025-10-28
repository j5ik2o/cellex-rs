use alloc::boxed::Box;

use cellex_utils_core_rs::sync::{
  shared::{Shared, SharedBound},
  ArcShared,
};

use crate::{
  api::{
    actor_scheduler::{
      actor_scheduler_handle::ActorSchedulerHandle, ready_queue_scheduler::ReadyQueueScheduler,
      ActorSchedulerHandleFactoryFn,
    },
    extensions::Extensions,
    guardian::GuardianStrategy,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::AnyMessage,
  },
};

/// Factory wrapper used to construct scheduler instances with consistent runtime configuration.
#[derive(Clone)]
pub struct ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  factory: ArcShared<ActorSchedulerHandleFactoryFn<MF>>,
}

impl<MF> ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
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
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  /// Returns a builder configured to create ready-queue-based schedulers.
  #[must_use]
  pub fn ready_queue() -> Self {
    Self::new(|mailbox_factory, extensions| Box::new(ReadyQueueScheduler::new(mailbox_factory, extensions)))
  }

  #[allow(dead_code)]
  /// Returns a builder that wires a custom guardian strategy into the ready-queue scheduler.
  pub fn with_strategy<Strat>(self, strategy: Strat) -> Self
  where
    Strat: GuardianStrategy<MF> + Clone + SharedBound, {
    let _ = self;
    Self::new(move |mailbox_factory, extensions| {
      Box::new(ReadyQueueScheduler::with_strategy(mailbox_factory, strategy.clone(), extensions))
    })
  }
}
