#![allow(dead_code)]

use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::{Element, QueueError};

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, SpawnError},
    actor_system::map_system::MapSystemShared,
    extensions::Extensions,
    failure_telemetry::FailureTelemetryShared,
    mailbox::{MailboxFactory, PriorityEnvelope},
    metrics::MetricsSinkShared,
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    actor_scheduler::{
      actor_scheduler::ActorScheduler, ready_queue_scheduler::ReadyQueueScheduler, ActorSchedulerSpawnContext,
    },
    supervision::{
      escalation::{FailureEventHandler, FailureEventListener},
      failure::FailureInfo,
      supervisor::Supervisor,
      telemetry::TelemetryObservationConfig,
    },
  },
  internal::guardian::{AlwaysRestart, GuardianStrategy},
};

/// Scheduler wrapper that executes actors immediately using the ReadyQueue scheduler logic.
///
/// This scheduler simply delegates to [`ReadyQueueScheduler`] but exposes a distinct builder entry
/// point.
pub struct ImmediateScheduler<M, MF, Strat = AlwaysRestart>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>, {
  inner: ReadyQueueScheduler<M, MF, Strat>,
}

impl<M, MF> ImmediateScheduler<M, MF, AlwaysRestart>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
{
  /// Creates a new immediate scheduler with the default guardian strategy.
  #[must_use]
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::new(mailbox_factory, extensions) }
  }
}

impl<M, MF, Strat> ImmediateScheduler<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  /// Creates a new immediate scheduler with a custom guardian strategy.
  #[must_use]
  pub fn with_strategy(mailbox_factory: MF, strategy: Strat, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::with_strategy(mailbox_factory, strategy, extensions) }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, MF, Strat> ActorScheduler<M, MF> for ImmediateScheduler<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
  Strat: GuardianStrategy<M, MF>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: ActorSchedulerSpawnContext<M, MF>,
  ) -> Result<PriorityActorRef<M, MF>, SpawnError<M>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
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

  fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, MF>, map_system: MapSystemShared<M>) {
    ReadyQueueScheduler::set_parent_guardian(&mut self.inner, control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    ReadyQueueScheduler::on_escalation(&mut self.inner, handler);
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.inner.take_escalations()
  }

  fn actor_count(&self) -> usize {
    self.inner.actor_count()
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.inner.drain_ready()
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.inner.dispatch_next().await
  }
}
