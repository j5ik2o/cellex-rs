#![allow(dead_code)]

use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::QueueError;

use crate::api::{
  actor::{actor_ref::PriorityActorRef, SpawnError},
  actor_scheduler::{
    actor_scheduler::ActorScheduler, ready_queue_scheduler::ReadyQueueScheduler, ActorSchedulerSpawnContext,
  },
  actor_system::map_system::MapSystemShared,
  extensions::Extensions,
  failure_telemetry::FailureTelemetryShared,
  guardian::{AlwaysRestart, GuardianStrategy},
  mailbox::{MailboxFactory, PriorityEnvelope},
  messaging::DynMessage,
  metrics::MetricsSinkShared,
  receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
  supervision::{
    escalation::{FailureEventHandler, FailureEventListener},
    failure::FailureInfo,
    supervisor::Supervisor,
    telemetry::TelemetryObservationConfig,
  },
};

/// Scheduler wrapper that executes actors immediately using the ReadyQueue scheduler logic.
///
/// This scheduler simply delegates to [`ReadyQueueScheduler`] but exposes a distinct builder entry
/// point.
pub struct ImmediateScheduler<MF, Strat = AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  inner: ReadyQueueScheduler<MF, Strat>,
}

impl<MF> ImmediateScheduler<MF, AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
{
  /// Creates a new immediate scheduler with the default guardian strategy.
  #[must_use]
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::new(mailbox_factory, extensions) }
  }
}

impl<MF, Strat> ImmediateScheduler<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  /// Creates a new immediate scheduler with a custom guardian strategy.
  #[must_use]
  pub fn with_strategy(mailbox_factory: MF, strategy: Strat, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::with_strategy(mailbox_factory, strategy, extensions) }
  }
}

#[async_trait::async_trait(?Send)]
impl<MF, Strat> ActorScheduler<MF> for ImmediateScheduler<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone,
  Strat: GuardianStrategy<MF>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<DynMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<PriorityActorRef<DynMessage, MF>, SpawnError<DynMessage>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, MF>>,
  ) {
    self.inner.set_receive_timeout_scheduler_factory_shared(factory);
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    ReadyQueueScheduler::set_root_event_listener(&mut self.inner, listener);
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    ReadyQueueScheduler::set_root_escalation_handler(&mut self.inner, handler);
  }

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    ReadyQueueScheduler::set_root_failure_telemetry(&mut self.inner, telemetry);
  }

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    ReadyQueueScheduler::set_root_observation_config(&mut self.inner, config);
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    ReadyQueueScheduler::set_metrics_sink(&mut self.inner, sink);
  }

  fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<DynMessage, MF>,
    map_system: MapSystemShared<DynMessage>,
  ) {
    ReadyQueueScheduler::set_parent_guardian(&mut self.inner, control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> + 'static>,
  ) {
    ReadyQueueScheduler::on_escalation(&mut self.inner, handler);
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.inner.take_escalations()
  }

  fn actor_count(&self) -> usize {
    self.inner.actor_count()
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.drain_ready()
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.dispatch_next().await
  }
}
