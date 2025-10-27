use std::{boxed::Box, vec::Vec};

use cellex_actor_core_rs::{
  api::{
    actor::{actor_ref::PriorityActorRef, SpawnError},
    actor_scheduler::{
      ready_queue_coordinator::{ReadyQueueCoordinator, SignalKey},
      ready_queue_scheduler::{ReadyQueueScheduler, ReadyQueueWorker},
      ActorScheduler, ActorSchedulerHandleBuilder, ActorSchedulerSpawnContext,
    },
    extensions::Extensions,
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{FailureTelemetryObservationConfig, FailureTelemetryShared},
      FailureInfo,
    },
    guardian::{AlwaysRestart, GuardianStrategy},
    metrics::MetricsSinkShared,
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::supervisor::Supervisor,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::{AnyMessage, MapSystemShared},
    supervision::FailureEventHandler,
  },
};
use cellex_utils_core_rs::{collections::queue::backend::QueueError, sync::ArcShared};
use tokio::task::yield_now;

/// Tokio scheduler wrapper.
///
/// Wraps the ReadyQueue-based
/// [`ReadyQueueScheduler`](cellex_actor_core_rs::api::actor_scheduler::ready_queue_scheduler::ReadyQueueScheduler)
/// and cooperatively
/// yields with `tokio::task::yield_now` after each dispatch.
pub struct TokioScheduler<MF, Strat = AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  inner: ReadyQueueScheduler<MF, Strat>,
}

impl<MF> TokioScheduler<MF, AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
{
  /// Builds the default configuration using the ReadyQueue scheduler.
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::new(mailbox_factory, extensions) }
  }
}

impl<MF, Strat> TokioScheduler<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  /// Builds a scheduler backed by a custom [`GuardianStrategy`].
  pub fn with_strategy(mailbox_factory: MF, strategy: Strat, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::with_strategy(mailbox_factory, strategy, extensions) }
  }
}

#[async_trait::async_trait(?Send)]
impl<MF, Strat> ActorScheduler<MF> for TokioScheduler<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  Strat: GuardianStrategy<MF>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<PriorityActorRef<AnyMessage, MF>, SpawnError<AnyMessage>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
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

  fn set_root_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
    ReadyQueueScheduler::set_root_observation_config(&mut self.inner, config);
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    ReadyQueueScheduler::set_metrics_sink(&mut self.inner, sink);
  }

  fn set_ready_queue_coordinator(&mut self, coordinator: Option<Box<dyn ReadyQueueCoordinator>>) {
    ReadyQueueScheduler::set_ready_queue_coordinator(&mut self.inner, coordinator);
  }

  fn notify_resume_signal(&mut self, key: SignalKey) -> bool {
    ReadyQueueScheduler::notify_resume_signal(&mut self.inner, key)
  }

  fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) {
    ReadyQueueScheduler::set_parent_guardian(&mut self.inner, control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static>,
  ) {
    ReadyQueueScheduler::on_escalation(&mut self.inner, handler);
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.inner.take_escalations()
  }

  fn actor_count(&self) -> usize {
    self.inner.actor_count()
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.inner.drain_ready()
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.inner.dispatch_next().await?;
    yield_now().await;
    Ok(())
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MF>>> {
    Some(self.inner.worker_handle())
  }
}

/// Utility that produces a scheduler builder configured for Tokio.
#[must_use]
pub fn tokio_scheduler_builder<MF>() -> ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  ActorSchedulerHandleBuilder::new(|mailbox_factory, extensions| {
    Box::new(TokioScheduler::<MF, AlwaysRestart>::new(mailbox_factory, extensions))
  })
}
