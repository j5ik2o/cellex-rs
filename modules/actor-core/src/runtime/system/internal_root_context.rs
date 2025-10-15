use alloc::boxed::Box;

use crate::api::actor::MailboxHandleFactoryStub;
use crate::runtime::context::InternalActorRef;
use crate::runtime::guardian::GuardianStrategy;
use crate::runtime::mailbox::traits::ActorRuntime;
use crate::runtime::scheduler::ChildNaming;
use crate::runtime::scheduler::SchedulerSpawnContext;
use crate::runtime::scheduler::SpawnError;
use crate::NoopSupervisor;
use crate::{Extensions, PriorityEnvelope, Supervisor};
use cellex_utils_core_rs::sync::Shared;
use cellex_utils_core_rs::{Element, QueueError};

use super::{InternalActorSystem, InternalProps};

pub(crate) struct InternalRootContext<'a, M, R, Strat>
where
  M: Element + 'static,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>, {
  pub(super) system: &'a mut InternalActorSystem<M, R, Strat>,
}

impl<'a, M, R, Strat> InternalRootContext<'a, M, R, Strat>
where
  M: Element + 'static,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  #[allow(dead_code)]
  pub fn spawn(&mut self, props: InternalProps<M, R>) -> Result<InternalActorRef<M, R>, SpawnError<M>> {
    self.spawn_with_supervisor(Box::new(NoopSupervisor), props, ChildNaming::Auto)
  }

  /// Spawns a child actor with an explicit supervisor and naming strategy.
  pub fn spawn_with_supervisor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, R>,
    child_naming: ChildNaming,
  ) -> Result<InternalActorRef<M, R>, SpawnError<M>> {
    let InternalProps {
      options,
      map_system,
      handler,
    } = props;

    let runtime = self.system.runtime.with_ref(|factory| factory.clone());
    let mut mailbox_factory = MailboxHandleFactoryStub::new(self.system.runtime.clone());
    mailbox_factory.set_metrics_sink(self.system.metrics_sink());
    let context = SchedulerSpawnContext {
      runtime,
      mailbox_handle_factory_stub: mailbox_factory,
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
