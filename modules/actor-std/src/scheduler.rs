use std::{boxed::Box, vec::Vec};

use cellex_actor_core_rs::api::{
  actor::{actor_ref::PriorityActorRef, SpawnError},
  actor_runtime::GenericActorRuntime,
  actor_scheduler::{
    ready_queue_scheduler::{ReadyQueueScheduler, ReadyQueueWorker},
    ActorScheduler, ActorSchedulerHandleBuilder, ActorSchedulerSpawnContext,
  },
  actor_system::map_system::MapSystemShared,
  extensions::Extensions,
  failure_telemetry::FailureTelemetryShared,
  guardian::{AlwaysRestart, GuardianStrategy},
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  messaging::AnyMessage,
  metrics::MetricsSinkShared,
  receive_timeout::{ReceiveTimeoutSchedulerFactoryProviderShared, ReceiveTimeoutSchedulerFactoryShared},
  supervision::{
    escalation::{FailureEventHandler, FailureEventListener},
    failure::FailureInfo,
    supervisor::Supervisor,
    telemetry::TelemetryObservationConfig,
  },
};
use cellex_utils_core_rs::{sync::ArcShared, QueueError};
use tokio::task::yield_now;

use crate::{TokioMailboxRuntime, TokioReceiveTimeoutDriver};

/// Tokio scheduler wrapper.
///
/// Wraps the ReadyQueue-based [`cellex_actor_core_rs::ReadyQueueScheduler`] and cooperatively
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

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    ReadyQueueScheduler::set_root_observation_config(&mut self.inner, config);
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    ReadyQueueScheduler::set_metrics_sink(&mut self.inner, sink);
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

/// Extension trait that installs Tokio-specific scheduler and timeout settings on
/// [`GenericActorRuntime`].
pub trait TokioActorRuntimeExt {
  /// Replaces the scheduler with the Tokio-backed implementation.
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxRuntime>;
}

impl TokioActorRuntimeExt for GenericActorRuntime<TokioMailboxRuntime> {
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxRuntime> {
    self.with_scheduler_builder(tokio_scheduler_builder()).with_receive_timeout_scheduler_factory_provider_shared_opt(
      Some(ReceiveTimeoutSchedulerFactoryProviderShared::new(TokioReceiveTimeoutDriver::new())),
    )
  }
}
