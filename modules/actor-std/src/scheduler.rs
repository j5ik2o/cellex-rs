use std::{boxed::Box, vec::Vec};

use cellex_actor_core_rs::{
  api::{
    actor::actor_ref::PriorityActorRef,
    actor_runtime::GenericActorRuntime,
    actor_system::map_system::MapSystemShared,
    extensions::Extensions,
    failure_telemetry::FailureTelemetryShared,
    mailbox::{MailboxFactory, PriorityEnvelope},
    metrics::MetricsSinkShared,
    receive_timeout::{ReceiveTimeoutSchedulerFactoryProviderShared, ReceiveTimeoutSchedulerFactoryShared},
    supervision::{
      escalation::{FailureEventHandler, FailureEventListener},
      failure::FailureInfo,
      supervisor::Supervisor,
      telemetry::TelemetryObservationConfig,
    },
  },
  internal::{
    guardian::{AlwaysRestart, GuardianStrategy},
    scheduler::{
      ActorScheduler, ReadyQueueScheduler, ReadyQueueWorker, SchedulerBuilder, SchedulerSpawnContext, SpawnError,
    },
  },
};
use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};
use tokio::task::yield_now;

/// Tokio 用スケジューララッパー。
///
/// ReadyQueue ベースの [`cellex_actor_core_rs::ReadyQueueScheduler`]
/// を内包しつつ、`tokio::task::yield_now` による協調切り替えを行う。
pub struct TokioScheduler<M, MF, Strat = AlwaysRestart>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>, {
  inner: ReadyQueueScheduler<M, MF, Strat>,
}

impl<M, MF> TokioScheduler<M, MF, AlwaysRestart>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
{
  /// ReadyQueue スケジューラを用いた既定構成を作成する。
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::new(mailbox_factory, extensions) }
  }
}

impl<M, MF, Strat> TokioScheduler<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  /// カスタム GuardianStrategy を適用した構成を作成する。
  pub fn with_strategy(mailbox_factory: MF, strategy: Strat, extensions: Extensions) -> Self {
    Self { inner: ReadyQueueScheduler::with_strategy(mailbox_factory, strategy, extensions) }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, MF, Strat> ActorScheduler<M, MF> for TokioScheduler<M, MF, Strat>
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
    context: SchedulerSpawnContext<M, MF>,
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
    self.inner.dispatch_next().await?;
    yield_now().await;
    Ok(())
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, MF>>> {
    Some(self.inner.worker_handle())
  }
}

/// Tokio 用スケジューラビルダーを生成するユーティリティ。
pub fn tokio_scheduler_builder<M, MF>() -> SchedulerBuilder<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  SchedulerBuilder::new(|mailbox_factory, extensions| {
    Box::new(TokioScheduler::<M, MF, AlwaysRestart>::new(mailbox_factory, extensions))
  })
}

use crate::{TokioMailboxRuntime, TokioReceiveTimeoutDriver};

/// 拡張トレイト: Tokio ランタイム向けスケジューラ／タイムアウト設定を `GenericActorRuntime`
/// に適用する。
pub trait TokioActorRuntimeExt {
  /// スケジューラを Tokio 実装へ差し替える。
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxRuntime>;
}

impl TokioActorRuntimeExt for GenericActorRuntime<TokioMailboxRuntime> {
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxRuntime> {
    self.with_scheduler_builder(tokio_scheduler_builder()).with_receive_timeout_driver(Some(
      ReceiveTimeoutSchedulerFactoryProviderShared::new(TokioReceiveTimeoutDriver::new()),
    ))
  }
}
