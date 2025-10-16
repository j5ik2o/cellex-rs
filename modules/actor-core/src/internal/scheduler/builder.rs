#![allow(missing_docs)]

use alloc::boxed::Box;

use crate::api::mailbox::PriorityEnvelope;
use crate::internal::guardian::GuardianStrategy;
#[cfg(any(test, feature = "test-support"))]
use crate::internal::scheduler::ImmediateScheduler;
use crate::{ActorScheduler, Extensions, MailboxRuntime, ReadyQueueScheduler};
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, SharedBound};

pub type SchedulerHandle<M, R> = Box<dyn ActorScheduler<M, R>>;
#[cfg(target_has_atomic = "ptr")]
type FactoryFn<M, R> = dyn Fn(R, Extensions) -> SchedulerHandle<M, R> + Send + Sync + 'static;
#[cfg(not(target_has_atomic = "ptr"))]
type FactoryFn<M, R> = dyn Fn(R, Extensions) -> SchedulerHandle<M, R> + 'static;

#[derive(Clone)]
pub struct SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  factory: ArcShared<FactoryFn<M, R>>,
}

impl<M, R> SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  #[cfg(any(test, feature = "test-support"))]
  #[allow(dead_code)]
  #[must_use]
  pub fn immediate() -> Self {
    Self::new(|mailbox_runtime, extensions| Box::new(ImmediateScheduler::new(mailbox_runtime, extensions)))
  }

  pub fn new<F>(factory: F) -> Self
  where
    F: Fn(R, Extensions) -> SchedulerHandle<M, R> + SharedBound + 'static, {
    let shared = ArcShared::new(factory);
    Self {
      factory: shared.into_dyn(|inner| inner as &FactoryFn<M, R>),
    }
  }

  pub fn build(&self, mailbox_runtime: R, extensions: Extensions) -> SchedulerHandle<M, R> {
    self.factory.with_ref(|factory| (factory)(mailbox_runtime, extensions))
  }
}
impl<M, R> SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  pub fn ready_queue() -> Self {
    Self::new(|mailbox_runtime, extensions| Box::new(ReadyQueueScheduler::new(mailbox_runtime, extensions)))
  }

  #[allow(dead_code)]
  pub fn with_strategy<Strat>(self, strategy: Strat) -> Self
  where
    Strat: GuardianStrategy<M, R> + Clone + Send + Sync, {
    let _ = self;
    Self::new(move |mailbox_runtime, extensions| {
      Box::new(ReadyQueueScheduler::with_strategy(
        mailbox_runtime,
        strategy.clone(),
        extensions,
      ))
    })
  }
}
