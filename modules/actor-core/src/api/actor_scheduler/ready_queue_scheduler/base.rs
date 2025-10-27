use alloc::{boxed::Box, vec::Vec};
use core::convert::Infallible;

use async_trait::async_trait;
use cellex_utils_core_rs::{collections::queue::backend::QueueError, sync::ArcShared};
use spin::Mutex;

use super::{
  common::ReadyQueueSchedulerCore,
  ready_event_hook::{ReadyEventHook, ReadyQueueHandle},
  ready_notifier::ReadyNotifier,
  ready_queue_context::ReadyQueueContext,
  ready_queue_state::ReadyQueueState,
  ready_queue_worker::ReadyQueueWorker,
};
use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, SpawnError},
    actor_scheduler::{ready_queue_scheduler::ReadyQueueWorkerImpl, ActorScheduler, ActorSchedulerSpawnContext},
    extensions::Extensions,
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{FailureTelemetryObservationConfig, FailureTelemetryShared},
      FailureInfo,
    },
    guardian::{AlwaysRestart, GuardianStrategy},
    metrics::{MetricsSinkShared, SuspensionClockShared},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::supervisor::Supervisor,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::{AnyMessage, MapSystemShared},
    supervision::FailureEventHandler,
  },
};

/// Ready-queue based actor scheduler that coordinates execution and escalation handling.
pub struct ReadyQueueScheduler<MF, Strat = AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  context: ArcShared<Mutex<ReadyQueueContext<MF, Strat>>>,
  state:   ArcShared<Mutex<ReadyQueueState>>,
}

#[allow(dead_code)]
impl<MF> ReadyQueueScheduler<MF, AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
{
  /// Creates a scheduler that uses the `AlwaysRestart` guardian strategy.
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self::with_strategy(mailbox_factory, AlwaysRestart, extensions)
  }

  /// Creates a scheduler with the provided guardian strategy.
  pub fn with_strategy<Strat>(
    mailbox_factory: MF,
    strategy: Strat,
    extensions: Extensions,
  ) -> ReadyQueueScheduler<MF, Strat>
  where
    Strat: GuardianStrategy<MF>, {
    let state = ArcShared::new(Mutex::new(ReadyQueueState::new()));
    let context = ReadyQueueContext {
      core:  ReadyQueueSchedulerCore::with_strategy(mailbox_factory, strategy, extensions),
      state: state.clone(),
    };
    ReadyQueueScheduler { context: ArcShared::new(Mutex::new(context)), state }
  }
}

impl<MF, Strat> ReadyQueueScheduler<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  /// Returns a handle that exposes ready-queue controls for cooperative workers.
  #[must_use]
  pub fn worker_handle(&self) -> ArcShared<dyn ReadyQueueWorker<MF>> {
    let shared = ArcShared::new(ReadyQueueWorkerImpl::<MF, Strat>::new(self.context.clone()));
    shared.into_dyn(|inner| inner as &dyn ReadyQueueWorker<MF>)
  }

  fn make_ready_handle(&self, index: usize) -> ReadyQueueHandle {
    let state = self.state.clone();
    let notifier = ArcShared::new(ReadyNotifier::new(state, index));
    notifier.into_dyn(|inner| inner as &dyn ReadyEventHook)
  }

  /// Spawns an actor and registers its mailbox with the ready queue.
  ///
  /// # Errors
  /// Returns [`SpawnError`] when the guardian or mailbox initialisation fails.
  pub fn spawn_actor_internal(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<PriorityActorRef<AnyMessage, MF>, SpawnError<AnyMessage>> {
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
  pub fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  ) {
    let mut ctx = self.context.lock();
    ctx.set_receive_timeout_scheduler_factory_shared(factory);
  }

  /// Installs a metrics sink tracking queue length and scheduling statistics.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    let mut ctx = self.context.lock();
    ctx.set_metrics_sink(sink);
  }

  /// Configures the suspension clock used to measure suspend durations.
  pub fn set_suspension_clock(&mut self, clock: SuspensionClockShared) {
    let mut ctx = self.context.lock();
    ctx.core.set_suspension_clock(clock);
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
  pub fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) {
    let mut ctx = self.context.lock();
    ctx.set_parent_guardian(control_ref, map_system);
  }

  /// Provides shared telemetry state used to publish failure diagnostics.
  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    let mut ctx = self.context.lock();
    ctx.set_root_failure_telemetry(telemetry);
  }

  /// Configures telemetry observation parameters such as sampling and filters.
  pub fn set_root_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
    let mut ctx = self.context.lock();
    ctx.set_root_observation_config(config);
  }

  /// Attaches a callback to react to escalation events raised by child actors.
  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static, {
    let mut ctx = self.context.lock();
    ctx.on_escalation(handler);
  }

  /// Drains any buffered escalations collected since the previous poll.
  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    let mut ctx = self.context.lock();
    ctx.take_escalations()
  }

  /// Returns the number of actors currently managed by the scheduler.
  #[must_use]
  pub fn actor_count(&self) -> usize {
    let ctx = self.context.lock();
    ctx.actor_count()
  }

  /// Processes queued ready actors and reports whether more work remains.
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue operations fail.
  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    let mut ctx = self.context.lock();
    ctx.drain_ready()
  }

  /// Drives the scheduler loop until the provided predicate returns `false`.
  ///
  /// # Errors
  /// Returns [`QueueError`] when dispatching an actor fails.
  pub async fn run_until<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.dispatch_next().await?;
    }
    Ok(())
  }

  /// Continuously dispatches work until an error causes the loop to terminate.
  ///
  /// # Errors
  /// Returns [`QueueError`] when dispatching an actor fails.
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>> {
    loop {
      self.dispatch_next().await?;
    }
  }

  /// Dispatches the next ready actor, waiting for mailbox signals when queues are empty.
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue processing fails.
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
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
impl<MF, Strat> ActorScheduler<MF> for ReadyQueueScheduler<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<PriorityActorRef<AnyMessage, MF>, SpawnError<AnyMessage>> {
    ReadyQueueScheduler::spawn_actor_internal(self, supervisor, context)
  }

  fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  ) {
    ReadyQueueScheduler::set_receive_timeout_scheduler_factory_shared(self, factory)
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

  fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) {
    ReadyQueueScheduler::set_parent_guardian(self, control_ref, map_system)
  }

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    ReadyQueueScheduler::set_root_failure_telemetry(self, telemetry)
  }

  fn set_root_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
    ReadyQueueScheduler::set_root_observation_config(self, config)
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static>,
  ) {
    ReadyQueueScheduler::on_escalation(self, handler)
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    ReadyQueueScheduler::take_escalations(self)
  }

  fn actor_count(&self) -> usize {
    ReadyQueueScheduler::actor_count(self)
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    ReadyQueueScheduler::drain_ready(self)
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    ReadyQueueScheduler::dispatch_next(self).await
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MF>>> {
    Some(self.worker_handle())
  }
}
