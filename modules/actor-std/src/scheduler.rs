use std::boxed::Box;
use std::vec::Vec;

use cellex_actor_core_rs::{
  ActorRuntimeBundle, ActorScheduler, AlwaysRestart, Extensions, FailureEventHandler, FailureEventListener,
  FailureInfo, GuardianStrategy, InternalActorRef, MailboxFactory, MapSystemShared, MetricsSinkShared,
  PriorityEnvelope, PriorityScheduler, ReceiveTimeoutFactoryShared, SchedulerBuilder, SchedulerSpawnContext,
  Supervisor,
};
use cellex_utils_std_rs::{Element, QueueError};
use tokio::task::yield_now;

/// Tokio 用スケジューララッパー。
///
/// 既存の [`PriorityScheduler`] を内包しつつ、`tokio::task::yield_now` による協調切り替えを行う。
pub struct TokioScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, ActorRuntimeBundle<R>>, {
  inner: PriorityScheduler<M, ActorRuntimeBundle<R>, Strat>,
}

impl<M, R> TokioScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
{
  /// `PriorityScheduler` を用いた既定構成を作成する。
  pub fn new(runtime: ActorRuntimeBundle<R>, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::new(runtime, extensions),
    }
  }
}

impl<M, R, Strat> TokioScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, ActorRuntimeBundle<R>>,
{
  /// カスタム GuardianStrategy を適用した構成を作成する。
  pub fn with_strategy(runtime: ActorRuntimeBundle<R>, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, ActorRuntimeBundle<R>> for TokioScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, ActorRuntimeBundle<R>>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, ActorRuntimeBundle<R>>,
  ) -> Result<InternalActorRef<M, ActorRuntimeBundle<R>>, QueueError<PriorityEnvelope<M>>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, ActorRuntimeBundle<R>>>) {
    self.inner.set_receive_timeout_factory(factory);
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    PriorityScheduler::set_root_event_listener(&mut self.inner, listener);
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    PriorityScheduler::set_root_escalation_handler(&mut self.inner, handler);
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    PriorityScheduler::set_metrics_sink(&mut self.inner, sink);
  }

  fn set_parent_guardian(
    &mut self,
    control_ref: InternalActorRef<M, ActorRuntimeBundle<R>>,
    map_system: MapSystemShared<M>,
  ) {
    PriorityScheduler::set_parent_guardian(&mut self.inner, control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    PriorityScheduler::on_escalation(&mut self.inner, handler);
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
pub fn tokio_scheduler_builder<M, R>() -> SchedulerBuilder<M, ActorRuntimeBundle<R>>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  SchedulerBuilder::new(|runtime, extensions| Box::new(TokioScheduler::new(runtime, extensions)))
}

/// `ActorRuntimeBundle` に Tokio スケジューラを組み込むための拡張トレイト。
pub trait ActorRuntimeBundleTokioExt<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<cellex_actor_core_rs::DynMessage>>: Clone,
  R::Signal: Clone, {
  /// スケジューラを Tokio 実装へ差し替える。
  fn with_tokio_scheduler(self) -> ActorRuntimeBundle<R>;
}

impl<R> ActorRuntimeBundleTokioExt<R> for ActorRuntimeBundle<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<cellex_actor_core_rs::DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn with_tokio_scheduler(self) -> ActorRuntimeBundle<R> {
    self.with_scheduler_builder(tokio_scheduler_builder())
  }
}
