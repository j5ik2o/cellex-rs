use std::boxed::Box;
use std::vec::Vec;

use cellex_actor_core_rs::{
  ActorScheduler, AlwaysRestart, Extensions, FailureEventHandler, FailureEventListener, FailureInfo,
  FailureTelemetryShared, GuardianStrategy, InternalActorRef, MailboxRuntime, MapSystemShared, MetricsSinkShared,
  PriorityEnvelope, ReadyQueueScheduler, ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared, RuntimeEnv,
  SchedulerBuilder, SchedulerSpawnContext, Supervisor, TelemetryObservationConfig,
};
use cellex_utils_std_rs::{Element, QueueError};
use tokio::task::yield_now;

/// Tokio 用スケジューララッパー。
///
/// ReadyQueue ベースの [`cellex_actor_core_rs::ReadyQueueScheduler`] を内包しつつ、`tokio::task::yield_now` による協調切り替えを行う。
pub struct TokioScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, RuntimeEnv<R>>, {
  inner: ReadyQueueScheduler<M, RuntimeEnv<R>, Strat>,
}

impl<M, R> TokioScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  /// `PriorityScheduler` を用いた既定構成を作成する。
  pub fn new(runtime: RuntimeEnv<R>, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::new(runtime, extensions),
    }
  }
}

impl<M, R, Strat> TokioScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, RuntimeEnv<R>>,
{
  /// カスタム GuardianStrategy を適用した構成を作成する。
  pub fn with_strategy(runtime: RuntimeEnv<R>, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, RuntimeEnv<R>> for TokioScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, RuntimeEnv<R>>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, RuntimeEnv<R>>,
  ) -> Result<InternalActorRef<M, RuntimeEnv<R>>, QueueError<PriorityEnvelope<M>>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, RuntimeEnv<R>>>) {
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

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, RuntimeEnv<R>>, map_system: MapSystemShared<M>) {
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
}

/// Tokio 用スケジューラビルダーを生成するユーティリティ。
pub fn tokio_scheduler_builder<M, R>() -> SchedulerBuilder<M, RuntimeEnv<R>>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  SchedulerBuilder::new(|runtime, extensions| Box::new(TokioScheduler::<M, R, AlwaysRestart>::new(runtime, extensions)))
}

use crate::{TokioMailboxRuntime, TokioReceiveTimeoutDriver};

/// 拡張トレイト: Tokio ランタイム向けスケジューラ／タイムアウト設定を `ActorRuntimeBundle` に適用する。
pub trait ActorRuntimeBundleTokioExt {
  /// スケジューラを Tokio 実装へ差し替える。
  fn with_tokio_scheduler(self) -> RuntimeEnv<TokioMailboxRuntime>;
}

impl ActorRuntimeBundleTokioExt for RuntimeEnv<TokioMailboxRuntime> {
  fn with_tokio_scheduler(self) -> RuntimeEnv<TokioMailboxRuntime> {
    self
      .with_scheduler_builder(tokio_scheduler_builder())
      .with_receive_timeout_driver(Some(ReceiveTimeoutDriverShared::new(TokioReceiveTimeoutDriver::new())))
  }
}
