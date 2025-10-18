use alloc::boxed::Box;

use cellex_utils_core_rs::{
  sync::{ArcShared, Shared},
  QueueError,
};
use spin::RwLock;

use super::InternalActorSystem;
use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ChildNaming, SpawnError},
    actor_runtime::{ActorRuntime, MailboxOf},
    actor_scheduler::ActorSchedulerSpawnContext,
    extensions::Extensions,
    guardian::GuardianStrategy,
    mailbox::{MailboxFactory, PriorityEnvelope},
    messaging::DynMessage,
    process::{pid::Pid, process_registry::ProcessRegistry},
    supervision::supervisor::{NoopSupervisor, Supervisor},
  },
  internal::actor::InternalProps,
};

pub(crate) struct InternalRootContext<'a, AR, Strat>
where
  AR: ActorRuntime + Clone + 'static,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>, {
  pub(super) system: &'a mut InternalActorSystem<AR, Strat>,
}

impl<'a, AR, Strat> InternalRootContext<'a, AR, Strat>
where
  AR: ActorRuntime + Clone + 'static,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>,
{
  #[allow(dead_code)]
  pub fn spawn(
    &mut self,
    props: InternalProps<MailboxOf<AR>>,
  ) -> Result<PriorityActorRef<DynMessage, MailboxOf<AR>>, SpawnError<DynMessage>> {
    let pid_slot = ArcShared::new(RwLock::new(None));
    self.spawn_with_supervisor(Box::new(NoopSupervisor), props, ChildNaming::Auto, pid_slot)
  }

  /// Spawns a child actor with an explicit supervisor and naming strategy.
  pub fn spawn_with_supervisor(
    &mut self,
    supervisor: Box<dyn Supervisor<DynMessage>>,
    props: InternalProps<MailboxOf<AR>>,
    child_naming: ChildNaming,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
  ) -> Result<PriorityActorRef<DynMessage, MailboxOf<AR>>, SpawnError<DynMessage>> {
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
  ) -> ArcShared<ProcessRegistry<PriorityActorRef<DynMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<DynMessage>>>>
  {
    self.system.process_registry()
  }

  #[deprecated(since = "3.1.0", note = "dispatch_next / run_until を使用してください")]
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    while self.system.scheduler.drain_ready()? {}
    Ok(())
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.system.scheduler.dispatch_next().await
  }

  pub fn extensions(&self) -> Extensions {
    self.system.extensions()
  }
}
