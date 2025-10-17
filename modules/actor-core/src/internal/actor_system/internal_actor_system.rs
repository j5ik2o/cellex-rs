use core::convert::Infallible;

use super::InternalRootContext;
use crate::api::actor_runtime::{ActorRuntime, MailboxOf};
use crate::api::extensions::Extensions;
use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::actor_system::internal_actor_system_config::InternalActorSystemConfig;
use crate::internal::guardian::{AlwaysRestart, GuardianStrategy};
use crate::internal::metrics::MetricsSinkShared;
use crate::internal::scheduler::ReadyQueueWorker;
use crate::internal::scheduler::SchedulerBuilder;
use crate::internal::scheduler::SchedulerHandle;
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, QueueError};
use core::marker::PhantomData;

pub(crate) struct InternalActorSystem<M, R, Strat = AlwaysRestart>
where
  M: Element + 'static,
  R: ActorRuntime + Clone + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  <MailboxOf<R> as MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxRuntime>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<R>>, {
  pub(super) scheduler: SchedulerHandle<M, MailboxOf<R>>,
  #[allow(dead_code)]
  pub(super) actor_runtime_shared: ArcShared<R>,
  pub(super) mailbox_runtime_shared: ArcShared<MailboxOf<R>>,
  extensions: Extensions,
  #[allow(dead_code)]
  metrics_sink: Option<MetricsSinkShared>,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<M, R> InternalActorSystem<M, R, AlwaysRestart>
where
  M: Element,
  R: ActorRuntime + Clone,
  MailboxOf<R>: MailboxRuntime + Clone,
  <MailboxOf<R> as MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxRuntime>::Signal: Clone,
{
  pub fn new(actor_runtime: R) -> Self {
    Self::new_with_config(actor_runtime, InternalActorSystemConfig::default())
  }

  pub fn new_with_config(actor_runtime: R, config: InternalActorSystemConfig<M, R>) -> Self {
    let scheduler_builder = ArcShared::new(SchedulerBuilder::<M, MailboxOf<R>>::ready_queue());
    Self::new_with_settings_and_builder(actor_runtime, &scheduler_builder, config)
  }

  pub fn new_with_settings_and_builder(
    actor_runtime: R,
    scheduler_builder: &ArcShared<SchedulerBuilder<M, MailboxOf<R>>>,
    config: InternalActorSystemConfig<M, R>,
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
    let mailbox_runtime_shared = actor_runtime_shared.with_ref(|rt| rt.mailbox_runtime_shared());
    let mailbox_runtime_for_scheduler = mailbox_runtime_shared.with_ref(|mr| mr.clone());
    let mut scheduler = scheduler_builder.build(mailbox_runtime_for_scheduler, extensions.clone());
    scheduler.set_root_event_listener(root_event_listener);
    scheduler.set_root_escalation_handler(root_escalation_handler);
    scheduler.set_root_failure_telemetry(root_failure_telemetry);
    scheduler.set_root_observation_config(root_observation_config);
    scheduler.set_receive_timeout_factory(receive_timeout_factory);
    scheduler.set_metrics_sink(metrics_sink.clone());
    Self {
      scheduler,
      actor_runtime_shared,
      mailbox_runtime_shared,
      extensions,
      metrics_sink,
      _strategy: PhantomData,
    }
  }
}

impl<M, R, Strat> InternalActorSystem<M, R, Strat>
where
  M: Element,
  R: ActorRuntime + Clone,
  MailboxOf<R>: MailboxRuntime + Clone,
  <MailboxOf<R> as MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
  <MailboxOf<R> as MailboxRuntime>::Signal: Clone,
  Strat: GuardianStrategy<M, MailboxOf<R>>,
{
  #[allow(clippy::missing_const_for_fn)]
  pub fn root_context(&mut self) -> InternalRootContext<'_, M, R, Strat> {
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
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, MailboxOf<R>>>> {
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
