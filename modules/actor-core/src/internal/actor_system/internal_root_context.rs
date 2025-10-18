use alloc::boxed::Box;

use cellex_utils_core_rs::{
  sync::{ArcShared, Shared},
  Element, QueueError,
};
use spin::RwLock;

use super::InternalActorSystem;
use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ChildNaming, SpawnError},
    actor_runtime::{ActorRuntime, MailboxOf},
    actor_scheduler::ActorSchedulerSpawnContext,
    extensions::Extensions,
    mailbox::{MailboxFactory, PriorityEnvelope},
    process::{pid::Pid, process_registry::ProcessRegistry},
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
    let pid_slot = ArcShared::new(RwLock::new(None));
    self.spawn_with_supervisor(Box::new(NoopSupervisor), props, ChildNaming::Auto, pid_slot)
  }

  /// Spawns a child actor with an explicit supervisor and naming strategy.
  pub fn spawn_with_supervisor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, MailboxOf<AR>>,
    child_naming: ChildNaming,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
  ) -> Result<PriorityActorRef<M, MailboxOf<AR>>, SpawnError<M>> {
    let InternalProps { options, map_system, handler } = props;

    let mailbox_factory = self.system.mailbox_factory_shared.with_ref(|mailbox| mailbox.clone());
    let mailbox_factory_shared = self.system.mailbox_factory_shared.clone();
    let context = ActorSchedulerSpawnContext {
      mailbox_factory,
      mailbox_factory_shared,
      map_system,
      mailbox_options: options,
      handler,
      child_naming,
      process_registry: self.system.process_registry(),
      actor_pid_slot: pid_slot,
    };
    self.system.scheduler.spawn_actor(supervisor, context)
  }

  pub fn process_registry(
    &self,
  ) -> ArcShared<ProcessRegistry<PriorityActorRef<M, MailboxOf<AR>>, ArcShared<PriorityEnvelope<M>>>> {
    self.system.process_registry()
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
