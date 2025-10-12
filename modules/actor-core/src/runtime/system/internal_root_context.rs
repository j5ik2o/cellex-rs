use alloc::boxed::Box;

use crate::api::actor::MailboxHandleFactoryStub;
use crate::runtime::context::InternalActorRef;
use crate::runtime::guardian::GuardianStrategy;
use crate::runtime::scheduler::SchedulerSpawnContext;
use crate::NoopSupervisor;
use crate::{Extensions, MailboxFactory, PriorityEnvelope, Supervisor};
use cellex_utils_core_rs::sync::Shared;
use cellex_utils_core_rs::{Element, QueueError};

use super::{InternalActorSystem, InternalProps};

pub(crate) struct InternalRootContext<'a, M, R, Strat>
where
  M: Element + 'static,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>, {
  pub(super) system: &'a mut InternalActorSystem<M, R, Strat>,
}

impl<'a, M, R, Strat> InternalRootContext<'a, M, R, Strat>
where
  M: Element + 'static,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  #[allow(dead_code)]
  pub fn spawn(
    &mut self,
    props: InternalProps<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    self.spawn_with_supervisor(Box::new(NoopSupervisor), props)
  }

  pub fn spawn_with_supervisor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    let InternalProps {
      options,
      map_system,
      handler,
    } = props;

    let runtime = self.system.runtime.with_ref(|factory| factory.clone());
    let mut mailbox_factory = MailboxHandleFactoryStub::new(self.system.runtime.clone());
    mailbox_factory.set_metrics_sink(self.system.metrics_sink());
    let mailbox_spawner = mailbox_factory.priority_spawner();
    let mailbox = mailbox_spawner.spawn_mailbox(options);
    let context = SchedulerSpawnContext {
      runtime,
      mailbox_factory,
      map_system,
      mailbox,
      handler,
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
