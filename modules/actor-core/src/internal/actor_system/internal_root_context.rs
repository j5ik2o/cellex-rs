use alloc::boxed::Box;

use cellex_utils_core_rs::{sync::Shared, Element, QueueError};

use super::InternalActorSystem;
use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ChildNaming, SpawnError},
    actor_runtime::{ActorRuntime, MailboxOf},
    extensions::Extensions,
    mailbox::{MailboxFactory, PriorityEnvelope},
    scheduler::SchedulerSpawnContext,
    supervision::supervisor::{NoopSupervisor, Supervisor},
  },
  internal::{actor::InternalProps, guardian::GuardianStrategy},
};

pub(crate) struct InternalRootContext<'a, M, AR, Strat>
where
  M: Element + 'static,
  AR: ActorRuntime + Clone + 'static,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<AR>>, {
  pub(super) system: &'a mut InternalActorSystem<M, AR, Strat>,
}

impl<'a, M, AR, Strat> InternalRootContext<'a, M, AR, Strat>
where
  M: Element + 'static,
  AR: ActorRuntime + Clone + 'static,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<AR>>,
{
  #[allow(dead_code)]
  pub fn spawn(
    &mut self,
    props: InternalProps<M, MailboxOf<AR>>,
  ) -> Result<PriorityActorRef<M, MailboxOf<AR>>, SpawnError<M>> {
    self.spawn_with_supervisor(Box::new(NoopSupervisor), props, ChildNaming::Auto)
  }

  /// Spawns a child actor with an explicit supervisor and naming strategy.
  pub fn spawn_with_supervisor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, MailboxOf<AR>>,
    child_naming: ChildNaming,
  ) -> Result<PriorityActorRef<M, MailboxOf<AR>>, SpawnError<M>> {
    let InternalProps { options, map_system, handler } = props;

    let mailbox_factory = self.system.mailbox_factory_shared.with_ref(|mailbox| mailbox.clone());
    let mailbox_factory_shared = self.system.mailbox_factory_shared.clone();
    let context = SchedulerSpawnContext {
      mailbox_factory,
      mailbox_factory_shared,
      map_system,
      mailbox_options: options,
      handler,
      child_naming,
    };
    self.system.scheduler.spawn_actor(supervisor, context)
  }

  #[deprecated(since = "3.1.0", note = "dispatch_next / run_until を使用してください")]
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    while self.system.scheduler.drain_ready()? {}
    Ok(())
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.system.scheduler.dispatch_next().await
  }

  pub fn extensions(&self) -> Extensions {
    self.system.extensions()
  }
}
