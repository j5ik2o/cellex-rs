use core::{convert::Infallible, marker::PhantomData};

use cellex_utils_core_rs::{
  sync::{ArcShared, Shared},
  QueueError,
};

use super::InternalRootContext;
use crate::{
  api::{
    actor::actor_ref::PriorityActorRef,
    actor_runtime::{ActorRuntime, MailboxOf},
    actor_scheduler::{ActorSchedulerHandle, ActorSchedulerHandleBuilder, ReadyQueueWorker},
    extensions::Extensions,
    guardian::{AlwaysRestart, GuardianStrategy},
    mailbox::{MailboxFactory, PriorityEnvelope},
    messaging::DynMessage,
    metrics::MetricsSinkShared,
    process::{
      pid::{NodeId, SystemId},
      process_registry::ProcessRegistry,
    },
  },
  internal::actor_system::internal_actor_system_config::InternalActorSystemConfig,
};

pub(crate) struct InternalActorSystem<AR, Strat = AlwaysRestart>
where
  AR: ActorRuntime + Clone + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>, {
  pub(super) scheduler: ActorSchedulerHandle<MailboxOf<AR>>,
  #[allow(dead_code)]
  pub(super) actor_runtime_shared: ArcShared<AR>,
  pub(super) mailbox_factory_shared: ArcShared<MailboxOf<AR>>,
  extensions: Extensions,
  #[allow(dead_code)]
  metrics_sink: Option<MetricsSinkShared>,
  pub(super) process_registry:
    ArcShared<ProcessRegistry<PriorityActorRef<DynMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<DynMessage>>>>,
  #[allow(dead_code)]
  pub(super) system_id: SystemId,
  #[allow(dead_code)]
  pub(super) node_id: Option<NodeId>,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<AR> InternalActorSystem<AR, AlwaysRestart>
where
  AR: ActorRuntime + Clone,
  MailboxOf<AR>: MailboxFactory + Clone,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
{
  pub fn new(actor_runtime: AR) -> Self {
    Self::new_with_config(actor_runtime, InternalActorSystemConfig::default())
  }

  pub fn new_with_config(actor_runtime: AR, config: InternalActorSystemConfig<AR>) -> Self {
    let scheduler_builder = ArcShared::new(ActorSchedulerHandleBuilder::<MailboxOf<AR>>::ready_queue());
    Self::new_with_config_and_builder(actor_runtime, &scheduler_builder, config)
  }

  pub fn new_with_config_and_builder(
    actor_runtime: AR,
    scheduler_builder: &ArcShared<ActorSchedulerHandleBuilder<MailboxOf<AR>>>,
    config: InternalActorSystemConfig<AR>,
  ) -> Self {
    let InternalActorSystemConfig {
      root_event_listener,
      root_escalation_handler,
      receive_timeout_factory,
      metrics_sink,
      root_failure_telemetry,
      root_observation_config,
      extensions,
      system_id,
      node_id,
    } = config;
    let actor_runtime_shared = ArcShared::new(actor_runtime);
    let mailbox_factory_shared = actor_runtime_shared.with_ref(|rt| rt.mailbox_factory_shared());
    let mailbox_factory_for_scheduler = mailbox_factory_shared.with_ref(|mr| mr.clone());
    let mut scheduler = scheduler_builder.build(mailbox_factory_for_scheduler, extensions.clone());
    scheduler.set_root_event_listener(root_event_listener);
    scheduler.set_root_escalation_handler(root_escalation_handler);
    scheduler.set_root_failure_telemetry(root_failure_telemetry);
    scheduler.set_root_observation_config(root_observation_config);
    scheduler.set_receive_timeout_scheduler_factory_shared(receive_timeout_factory);
    scheduler.set_metrics_sink(metrics_sink.clone());
    let process_registry = ArcShared::new(ProcessRegistry::new(system_id.clone(), node_id.clone()));
    Self {
      scheduler,
      actor_runtime_shared,
      mailbox_factory_shared,
      extensions,
      metrics_sink,
      process_registry,
      system_id,
      node_id,
      _strategy: PhantomData,
    }
  }
}

impl<AR, Strat> InternalActorSystem<AR, Strat>
where
  AR: ActorRuntime + Clone,
  MailboxOf<AR>: MailboxFactory + Clone,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>,
{
  #[allow(clippy::missing_const_for_fn)]
  pub fn root_context(&mut self) -> InternalRootContext<'_, AR, Strat> {
    InternalRootContext { system: self }
  }

  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    F: FnMut() -> bool, {
    self.run_until_impl(should_continue).await
  }

  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    loop {
      self.scheduler.dispatch_next().await?;
    }
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.scheduler.dispatch_next().await
  }

  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<DynMessage>>> {
    self.scheduler.drain_ready()
  }

  pub fn run_until_idle<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      let processed = self.drain_ready()?;
      if !processed {
        break;
      }
    }
    Ok(())
  }

  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  #[allow(dead_code)]
  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>> {
    self.scheduler.ready_queue_worker()
  }

  #[must_use]
  pub fn process_registry(
    &self,
  ) -> ArcShared<ProcessRegistry<PriorityActorRef<DynMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<DynMessage>>>>
  {
    self.process_registry.clone()
  }

  #[allow(dead_code)]
  #[must_use]
  pub fn system_id(&self) -> &SystemId {
    &self.system_id
  }

  #[allow(dead_code)]
  #[must_use]
  pub fn node_id(&self) -> Option<&NodeId> {
    self.node_id.as_ref()
  }

  async fn run_until_impl<F>(
    &mut self,
    mut should_continue: F,
  ) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.scheduler.dispatch_next().await?;
    }
    Ok(())
  }
}
