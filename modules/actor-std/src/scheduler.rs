use std::boxed::Box;
use std::vec::Vec;

use cellex_actor_core_rs::api::actor::actor_ref::PriorityActorRef;
use cellex_actor_core_rs::api::actor_runtime::GenericActorRuntime;
use cellex_actor_core_rs::api::extensions::Extensions;
use cellex_actor_core_rs::api::mailbox::MailboxFactory;
use cellex_actor_core_rs::api::mailbox::PriorityEnvelope;
use cellex_actor_core_rs::api::supervision::escalation::{FailureEventHandler, FailureEventListener};
use cellex_actor_core_rs::api::supervision::failure::FailureInfo;
use cellex_actor_core_rs::api::supervision::supervisor::Supervisor;
use cellex_actor_core_rs::api::supervision::telemetry::TelemetryObservationConfig;
use cellex_actor_core_rs::internal::guardian::{AlwaysRestart, GuardianStrategy};
use cellex_actor_core_rs::internal::metrics::MetricsSinkShared;
use cellex_actor_core_rs::internal::scheduler::{
  ActorScheduler, ReadyQueueScheduler, ReadyQueueWorker, SchedulerBuilder, SchedulerSpawnContext, SpawnError,
};
use cellex_actor_core_rs::shared::failure_telemetry::FailureTelemetryShared;
use cellex_actor_core_rs::shared::map_system::MapSystemShared;
use cellex_actor_core_rs::shared::receive_timeout::{
  ReceiveTimeoutSchedulerFactoryProviderShared, ReceiveTimeoutSchedulerFactoryShared,
};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};
use tokio::task::yield_now;

/// Tokio 用スケジューララッパー。
///
/// ReadyQueue ベースの [`cellex_actor_core_rs::ReadyQueueScheduler`] を内包しつつ、`tokio::task::yield_now` による協調切り替えを行う。
pub struct TokioScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  inner: ReadyQueueScheduler<M, R, Strat>,
}

impl<M, R> TokioScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
{
  /// ReadyQueue スケジューラを用いた既定構成を作成する。
  pub fn new(mailbox_runtime: R, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::new(mailbox_runtime, extensions),
    }
  }
}

impl<M, R, Strat> TokioScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  /// カスタム GuardianStrategy を適用した構成を作成する。
  pub fn with_strategy(mailbox_runtime: R, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::with_strategy(mailbox_runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, R> for TokioScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<PriorityActorRef<M, R>, SpawnError<M>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, R>>) {
    self.inner.set_receive_timeout_factory(factory);
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

  fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, R>, map_system: MapSystemShared<M>) {
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

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, R>>> {
    Some(self.inner.worker_handle())
  }
}

/// Tokio 用スケジューラビルダーを生成するユーティリティ。
pub fn tokio_scheduler_builder<M, R>() -> SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  SchedulerBuilder::new(|mailbox_runtime, extensions| {
    Box::new(TokioScheduler::<M, R, AlwaysRestart>::new(mailbox_runtime, extensions))
  })
}

use crate::{TokioMailboxRuntime, TokioReceiveTimeoutDriver};

/// 拡張トレイト: Tokio ランタイム向けスケジューラ／タイムアウト設定を `GenericActorRuntime` に適用する。
pub trait TokioActorRuntimeExt {
  /// スケジューラを Tokio 実装へ差し替える。
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxRuntime>;
}

impl TokioActorRuntimeExt for GenericActorRuntime<TokioMailboxRuntime> {
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxRuntime> {
    self
      .with_scheduler_builder(tokio_scheduler_builder())
      .with_receive_timeout_driver(Some(ReceiveTimeoutSchedulerFactoryProviderShared::new(
        TokioReceiveTimeoutDriver::new(),
      )))
  }
}
