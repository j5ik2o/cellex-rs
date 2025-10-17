use alloc::boxed::Box;
use alloc::vec::Vec;
use core::convert::Infallible;

use async_trait::async_trait;
use spin::Mutex;

use super::super::actor_scheduler::ActorScheduler;
use super::common::ReadyQueueSchedulerCore;
use super::ready_event_hook::{ReadyEventHook, ReadyQueueHandle};
use super::ready_notifier::ReadyNotifier;
use super::ready_queue_context::ReadyQueueContext;
use super::ready_queue_state::ReadyQueueState;
use super::ready_queue_worker::ReadyQueueWorker;
use crate::api::actor::actor_ref::PriorityActorRef;
use crate::api::extensions::Extensions;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::supervisor::Supervisor;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::guardian::{AlwaysRestart, GuardianStrategy};
use crate::internal::metrics::MetricsSinkShared;
use crate::internal::scheduler::ready_queue_scheduler::ReadyQueueWorkerImpl;
use crate::internal::scheduler::SchedulerSpawnContext;
use crate::internal::scheduler::SpawnError;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use crate::shared::map_system::MapSystemShared;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Ready-queue based actor scheduler that coordinates execution and escalation handling.
pub struct ReadyQueueScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  context: ArcShared<Mutex<ReadyQueueContext<M, R, Strat>>>,
  state: ArcShared<Mutex<ReadyQueueState>>,
}

#[allow(dead_code)]
impl<M, R> ReadyQueueScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
{
  /// Creates a scheduler that uses the `AlwaysRestart` guardian strategy.
  pub fn new(mailbox_runtime: R, extensions: Extensions) -> Self {
    Self::with_strategy(mailbox_runtime, AlwaysRestart, extensions)
  }

  /// Creates a scheduler with the provided guardian strategy.
  pub fn with_strategy<Strat>(
    mailbox_runtime: R,
    strategy: Strat,
    extensions: Extensions,
  ) -> ReadyQueueScheduler<M, R, Strat>
  where
    Strat: GuardianStrategy<M, R>, {
    let state = ArcShared::new(Mutex::new(ReadyQueueState::new()));
    let context = ReadyQueueContext {
      core: ReadyQueueSchedulerCore::with_strategy(mailbox_runtime, strategy, extensions),
      state: state.clone(),
    };
    ReadyQueueScheduler {
      context: ArcShared::new(Mutex::new(context)),
      state,
    }
  }
}

impl<M, R, Strat> ReadyQueueScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  /// Returns a handle that exposes ready-queue controls for cooperative workers.
  pub fn worker_handle(&self) -> ArcShared<dyn ReadyQueueWorker<M, R>> {
    let shared = ArcShared::new(ReadyQueueWorkerImpl::<M, R, Strat>::new(self.context.clone()));
    shared.into_dyn(|inner| inner as &dyn ReadyQueueWorker<M, R>)
  }

  fn make_ready_handle(&self, index: usize) -> ReadyQueueHandle {
    let state = self.state.clone();
    let notifier = ArcShared::new(ReadyNotifier::new(state, index));
    #[cfg(target_has_atomic = "ptr")]
    {
      notifier.into_dyn(|inner| inner as &(dyn ReadyEventHook + Send + Sync))
    }
    #[cfg(not(target_has_atomic = "ptr"))]
    {
      notifier.into_dyn(|inner| inner as &dyn ReadyEventHook)
    }
  }

  /// Spawns an actor and registers its mailbox with the ready queue.
  pub fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<PriorityActorRef<M, R>, SpawnError<M>> {
    let (actor_ref, index) = {
      let mut ctx = self.context.lock();
      ctx.spawn_actor(supervisor, context)?
    };

    let hook = self.make_ready_handle(index);
    {
      let mut ctx = self.context.lock();
      if let Some(cell) = ctx.actor_mut(index) {
        cell.set_scheduler_hook(Some(hook));
      }
      ctx.enqueue_ready(index);
    }

    Ok(actor_ref)
  }

  /// Configures the receive-timeout factory shared by all scheduled actors.
  pub fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, R>>) {
    let mut ctx = self.context.lock();
    ctx.set_receive_timeout_factory(factory);
  }

  /// Installs a metrics sink tracking queue length and scheduling statistics.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    let mut ctx = self.context.lock();
    ctx.set_metrics_sink(sink);
  }

  /// Sets the listener that receives failures propagated to the root supervisor.
  pub fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    let mut ctx = self.context.lock();
    ctx.set_root_event_listener(listener);
  }

  /// Registers the handler that processes root-level failure escalations.
  pub fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    let mut ctx = self.context.lock();
    ctx.set_root_escalation_handler(handler);
  }

  /// Wires the parent guardian controls needed for supervising newly spawned actors.
  pub fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, R>, map_system: MapSystemShared<M>) {
    let mut ctx = self.context.lock();
    ctx.set_parent_guardian(control_ref, map_system);
  }

  /// Provides shared telemetry state used to publish failure diagnostics.
  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    let mut ctx = self.context.lock();
    ctx.set_root_failure_telemetry(telemetry);
  }

  /// Configures telemetry observation parameters such as sampling and filters.
  pub fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    let mut ctx = self.context.lock();
    ctx.set_root_observation_config(config);
  }

  /// Attaches a callback to react to escalation events raised by child actors.
  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    let mut ctx = self.context.lock();
    ctx.on_escalation(handler);
  }

  /// Drains any buffered escalations collected since the previous poll.
  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    let mut ctx = self.context.lock();
    ctx.take_escalations()
  }

  /// Returns the number of actors currently managed by the scheduler.
  pub fn actor_count(&self) -> usize {
    let ctx = self.context.lock();
    ctx.actor_count()
  }

  /// Processes queued ready actors and reports whether more work remains.
  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    let mut ctx = self.context.lock();
    ctx.drain_ready()
  }

  /// Drives the scheduler loop until the provided predicate returns `false`.
  pub async fn run_until<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.dispatch_next().await?;
    }
    Ok(())
  }

  /// Continuously dispatches work until an error causes the loop to terminate.
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      self.dispatch_next().await?;
    }
  }

  /// Dispatches the next ready actor, waiting for mailbox signals when queues are empty.
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    loop {
      {
        let mut ctx = self.context.lock();
        if let Some(index) = ctx.dequeue_ready() {
          let processed = ctx.process_actor_pending(index)?;
          let has_pending = ctx.actor_has_pending(index);
          ctx.mark_idle(index, has_pending);
          if processed {
            return Ok(());
          }
          continue;
        }

        if ctx.drain_ready()? {
          return Ok(());
        }
      }

      let wait_future_opt = {
        let ctx = self.context.lock();
        ctx.wait_for_any_signal_future()
      };

      let Some(wait_future) = wait_future_opt else {
        return Ok(());
      };
      let index = wait_future.await;

      let ctx = self.context.lock();
      ctx.enqueue_ready(index);
    }
  }
}

#[async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, R> for ReadyQueueScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<PriorityActorRef<M, R>, SpawnError<M>> {
    ReadyQueueScheduler::spawn_actor(self, supervisor, context)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, R>>) {
    ReadyQueueScheduler::set_receive_timeout_factory(self, factory)
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    ReadyQueueScheduler::set_root_event_listener(self, listener)
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    ReadyQueueScheduler::set_root_escalation_handler(self, handler)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    ReadyQueueScheduler::set_metrics_sink(self, sink)
  }

  fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, R>, map_system: MapSystemShared<M>) {
    ReadyQueueScheduler::set_parent_guardian(self, control_ref, map_system)
  }

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    ReadyQueueScheduler::set_root_failure_telemetry(self, telemetry)
  }

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    ReadyQueueScheduler::set_root_observation_config(self, config)
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    ReadyQueueScheduler::on_escalation(self, handler)
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    ReadyQueueScheduler::take_escalations(self)
  }

  fn actor_count(&self) -> usize {
    ReadyQueueScheduler::actor_count(self)
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    ReadyQueueScheduler::drain_ready(self)
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    ReadyQueueScheduler::dispatch_next(self).await
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, R>>> {
    Some(self.worker_handle())
  }
}
