use alloc::boxed::Box;

use super::InternalActorSystem;
use crate::api::actor_runtime::{ActorRuntime, MailboxOf};
use crate::api::extensions::Extensions;
use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::supervision::supervisor::{NoopSupervisor, Supervisor};
use crate::internal::actor::{InternalActorRef, InternalProps};
use crate::internal::guardian::GuardianStrategy;
use crate::internal::scheduler::ChildNaming;
use crate::internal::scheduler::SchedulerSpawnContext;
use crate::internal::scheduler::SpawnError;
use cellex_utils_core_rs::sync::Shared;
use cellex_utils_core_rs::{Element, QueueError};

pub(crate) struct InternalRootContext<'a, M, R, Strat>
where
  M: Element + 'static,
  R: ActorRuntime + Clone + 'static,
  <MailboxOf<R> as MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxRuntime>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<R>>, {
  pub(super) system: &'a mut InternalActorSystem<M, R, Strat>,
}

impl<'a, M, R, Strat> InternalRootContext<'a, M, R, Strat>
where
  M: Element + 'static,
  R: ActorRuntime + Clone + 'static,
  <MailboxOf<R> as MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxRuntime>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<R>>,
{
  #[allow(dead_code)]
  pub fn spawn(
    &mut self,
    props: InternalProps<M, MailboxOf<R>>,
  ) -> Result<InternalActorRef<M, MailboxOf<R>>, SpawnError<M>> {
    self.spawn_with_supervisor(Box::new(NoopSupervisor), props, ChildNaming::Auto)
  }

  /// Spawns a child actor with an explicit supervisor and naming strategy.
  pub fn spawn_with_supervisor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, MailboxOf<R>>,
    child_naming: ChildNaming,
  ) -> Result<InternalActorRef<M, MailboxOf<R>>, SpawnError<M>> {
    let InternalProps {
      options,
      map_system,
      handler,
    } = props;

    let mailbox_runtime = self.system.mailbox_runtime_shared.with_ref(|mailbox| mailbox.clone());
    let mailbox_runtime_shared = self.system.mailbox_runtime_shared.clone();
    let context = SchedulerSpawnContext {
      mailbox_runtime,
      mailbox_runtime_shared,
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
