use core::{convert::Infallible, marker::PhantomData};

use cellex_utils_core_rs::{
  sync::{ArcShared, Shared},
  Element, QueueError,
};

use super::InternalRootContext;
use crate::{
  api::{
    actor_runtime::{ActorRuntime, MailboxOf},
    extensions::Extensions,
    mailbox::{MailboxFactory, PriorityEnvelope},
    metrics::MetricsSinkShared,
  },
  internal::{
    actor_system::internal_actor_system_config::InternalActorSystemConfig,
    guardian::{AlwaysRestart, GuardianStrategy},
    scheduler::{ReadyQueueWorker, SchedulerBuilder, SchedulerHandle},
  },
};

pub(crate) struct InternalActorSystem<M, AR, Strat = AlwaysRestart>
where
  M: Element + 'static,
  AR: ActorRuntime + Clone + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<AR>>, {
  pub(super) scheduler: SchedulerHandle<M, MailboxOf<AR>>,
  #[allow(dead_code)]
  pub(super) actor_runtime_shared: ArcShared<AR>,
  pub(super) mailbox_factory_shared: ArcShared<MailboxOf<AR>>,
  extensions: Extensions,
  #[allow(dead_code)]
  metrics_sink: Option<MetricsSinkShared>,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<M, AR> InternalActorSystem<M, AR, AlwaysRestart>
where
  M: Element,
  AR: ActorRuntime + Clone,
  MailboxOf<AR>: MailboxFactory + Clone,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
{
  pub fn new(actor_runtime: AR) -> Self {
    Self::new_with_config(actor_runtime, InternalActorSystemConfig::default())
  }

  pub fn new_with_config(actor_runtime: AR, config: InternalActorSystemConfig<M, AR>) -> Self {
    let scheduler_builder = ArcShared::new(SchedulerBuilder::<M, MailboxOf<AR>>::ready_queue());
    Self::new_with_settings_and_builder(actor_runtime, &scheduler_builder, config)
  }

  pub fn new_with_settings_and_builder(
    actor_runtime: AR,
    scheduler_builder: &ArcShared<SchedulerBuilder<M, MailboxOf<AR>>>,
    config: InternalActorSystemConfig<M, AR>,
  ) -> Self {
    let InternalActorSystemConfig {
      root_event_listener,
      root_escalation_handler,
      receive_timeout_factory,
      metrics_sink,
      root_failure_telemetry,
      root_observation_config,
      extensions,
    } = config;
    let actor_runtime_shared = ArcShared::new(actor_runtime);
    let mailbox_factory_shared = actor_runtime_shared.with_ref(|rt| rt.mailbox_factory_shared());
    let mailbox_factory_for_scheduler = mailbox_factory_shared.with_ref(|mr| mr.clone());
    let mut scheduler = scheduler_builder.build(mailbox_factory_for_scheduler, extensions.clone());
    scheduler.set_root_event_listener(root_event_listener);
    scheduler.set_root_escalation_handler(root_escalation_handler);
    scheduler.set_root_failure_telemetry(root_failure_telemetry);
    scheduler.set_root_observation_config(root_observation_config);
    scheduler.set_receive_timeout_factory(receive_timeout_factory);
    scheduler.set_metrics_sink(metrics_sink.clone());
    Self { scheduler, actor_runtime_shared, mailbox_factory_shared, extensions, metrics_sink, _strategy: PhantomData }
  }
}

impl<M, AR, Strat> InternalActorSystem<M, AR, Strat>
where
  M: Element,
  AR: ActorRuntime + Clone,
  MailboxOf<AR>: MailboxFactory + Clone,
  <MailboxOf<AR> as MailboxFactory>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<AR> as MailboxFactory>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<AR>>,
{
  #[allow(clippy::missing_const_for_fn)]
  pub fn root_context(&mut self) -> InternalRootContext<'_, M, AR, Strat> {
    InternalRootContext { system: self }
  }

  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    self.run_until_impl(should_continue).await
  }

  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      self.scheduler.dispatch_next().await?;
    }
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.scheduler.dispatch_next().await
  }

  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.scheduler.drain_ready()
  }

  pub fn run_until_idle<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
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
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, MailboxOf<AR>>>> {
    self.scheduler.ready_queue_worker()
  }

  async fn run_until_impl<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.scheduler.dispatch_next().await?;
    }
    Ok(())
  }
}
