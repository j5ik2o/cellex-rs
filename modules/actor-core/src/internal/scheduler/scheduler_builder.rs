use alloc::boxed::Box;

use crate::api::extensions::Extensions;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::guardian::GuardianStrategy;
use crate::internal::scheduler::actor_scheduler::ActorScheduler;
#[cfg(any(test, feature = "test-support"))]
use crate::internal::scheduler::immediate_scheduler::ImmediateScheduler;
use crate::internal::scheduler::ready_queue_scheduler::ReadyQueueScheduler;
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, SharedBound};

/// Type alias for boxed scheduler instances returned by builders.
pub type SchedulerHandle<M, MF> = Box<dyn ActorScheduler<M, MF>>;
#[cfg(target_has_atomic = "ptr")]
type FactoryFn<M, MF> = dyn Fn(MF, Extensions) -> SchedulerHandle<M, MF> + Send + Sync + 'static;
#[cfg(not(target_has_atomic = "ptr"))]
type FactoryFn<M, MF> = dyn Fn(MF, Extensions) -> SchedulerHandle<M, MF> + 'static;

/// Factory wrapper used to construct scheduler instances with consistent runtime configuration.
#[derive(Clone)]
pub struct SchedulerBuilder<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  factory: ArcShared<FactoryFn<M, MF>>,
}

impl<M, MF> SchedulerBuilder<M, MF>
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
    F: Fn(MF, Extensions) -> SchedulerHandle<M, MF> + SharedBound + 'static, {
    let shared = ArcShared::new(factory);
    Self {
      factory: shared.into_dyn(|inner| inner as &FactoryFn<M, MF>),
    }
  }

  /// Builds a scheduler using the stored factory and provided runtime components.
  pub fn build(&self, mailbox_factory: MF, extensions: Extensions) -> SchedulerHandle<M, MF> {
    self.factory.with_ref(|factory| (factory)(mailbox_factory, extensions))
  }
}
impl<M, MF> SchedulerBuilder<M, MF>
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
      Box::new(ReadyQueueScheduler::with_strategy(
        mailbox_factory,
        strategy.clone(),
        extensions,
      ))
    })
  }
}
