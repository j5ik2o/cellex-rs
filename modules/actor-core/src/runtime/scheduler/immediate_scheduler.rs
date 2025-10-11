#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::runtime::context::{ActorHandlerFn, InternalActorRef};
use crate::runtime::guardian::{AlwaysRestart, GuardianStrategy};
use crate::runtime::scheduler::actor_scheduler::ActorScheduler;
use crate::runtime::scheduler::priority_scheduler::PriorityScheduler;
use crate::{
  Extensions, FailureEventHandler, FailureEventListener, FailureInfo, MailboxFactory, MailboxOptions, MapSystemShared,
  PriorityEnvelope, ReceiveTimeoutFactoryShared, Supervisor,
};
use cellex_utils_core_rs::{Element, QueueError};

/// Scheduler wrapper that executes actors immediately using the existing priority scheduler logic.
///
/// This scheduler simply delegates to [`PriorityScheduler`] but exposes a distinct builder entry point.
pub(crate) struct ImmediateScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  inner: PriorityScheduler<M, R, Strat>,
}

impl<M, R> ImmediateScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
{
  pub fn new(runtime: R, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::new(runtime, extensions),
    }
  }
}

impl<M, R, Strat> ImmediateScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  pub fn with_strategy(runtime: R, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, R> for ImmediateScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    options: MailboxOptions,
    map_system: MapSystemShared<M>,
    handler: Box<ActorHandlerFn<M, R>>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    self.inner.spawn_actor(supervisor, options, map_system, handler)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    self.inner.set_receive_timeout_factory(factory);
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    PriorityScheduler::set_root_event_listener(&mut self.inner, listener);
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    PriorityScheduler::set_root_escalation_handler(&mut self.inner, handler);
  }

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    PriorityScheduler::set_parent_guardian(&mut self.inner, control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    PriorityScheduler::on_escalation(&mut self.inner, handler);
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.inner.take_escalations()
  }

  fn actor_count(&self) -> usize {
    self.inner.actor_count()
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.inner.drain_ready()
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.inner.dispatch_next().await
  }
}
