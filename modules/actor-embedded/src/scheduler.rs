#![cfg(feature = "embassy_executor")]

use alloc::boxed::Box;
use alloc::vec::Vec;

use cellex_actor_core_rs::{
  ActorRuntimeBundle, ActorScheduler, AlwaysRestart, Extensions, FailureEventHandler, FailureEventListener,
  FailureInfo, GuardianStrategy, InternalActorRef, MailboxFactory, MapSystemShared, MetricsSinkShared,
  PriorityEnvelope, PriorityScheduler, ReceiveTimeoutFactoryShared, SchedulerBuilder, SchedulerSpawnContext,
  Supervisor,
};
use cellex_utils_embedded_rs::Element;
use embassy_futures::yield_now;

/// Embassy 用スケジューラ。
///
/// 既存の [`PriorityScheduler`] をラップし、`embassy_futures::yield_now` による協調切り替えを提供する。
pub struct EmbassyScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, ActorRuntimeBundle<R>>, {
  inner: PriorityScheduler<M, ActorRuntimeBundle<R>, Strat>,
}

impl<M, R> EmbassyScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
{
  /// 既定の GuardianStrategy (`AlwaysRestart`) を用いた構成を作成する。
  pub fn new(runtime: ActorRuntimeBundle<R>, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::new(runtime, extensions),
    }
  }
}

impl<M, R, Strat> EmbassyScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, ActorRuntimeBundle<R>>,
{
  /// 任意の GuardianStrategy を適用した構成を作成する。
  pub fn with_strategy(runtime: ActorRuntimeBundle<R>, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, ActorRuntimeBundle<R>> for EmbassyScheduler<M, R, Strat>
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
  ) -> Result<InternalActorRef<M, ActorRuntimeBundle<R>>, cellex_utils_embedded_rs::QueueError<PriorityEnvelope<M>>> {
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
    handler: Box<
      dyn FnMut(&FailureInfo) -> Result<(), cellex_utils_embedded_rs::QueueError<PriorityEnvelope<M>>> + 'static,
    >,
  ) {
    PriorityScheduler::on_escalation(&mut self.inner, handler);
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.inner.take_escalations()
  }

  fn actor_count(&self) -> usize {
    self.inner.actor_count()
  }

  fn drain_ready(&mut self) -> Result<bool, cellex_utils_embedded_rs::QueueError<PriorityEnvelope<M>>> {
    self.inner.drain_ready()
  }

  async fn dispatch_next(&mut self) -> Result<(), cellex_utils_embedded_rs::QueueError<PriorityEnvelope<M>>> {
    self.inner.dispatch_next().await?;
    yield_now().await;
    Ok(())
  }
}

/// Embassy 用スケジューラビルダーを生成するユーティリティ。
pub fn embassy_scheduler_builder<M, R>() -> SchedulerBuilder<M, ActorRuntimeBundle<R>>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  SchedulerBuilder::new(|runtime, extensions| Box::new(EmbassyScheduler::new(runtime, extensions)))
}

/// `ActorRuntimeBundle` に Embassy スケジューラを組み込むための拡張トレイト。
pub trait ActorRuntimeBundleEmbassyExt<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<cellex_actor_core_rs::DynMessage>>: Clone,
  R::Signal: Clone, {
  /// スケジューラを Embassy 実装へ差し替える。
  fn with_embassy_scheduler(self) -> ActorRuntimeBundle<R>;
}

impl<R> ActorRuntimeBundleEmbassyExt<R> for ActorRuntimeBundle<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<cellex_actor_core_rs::DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn with_embassy_scheduler(self) -> ActorRuntimeBundle<R> {
    self.with_scheduler_builder(embassy_scheduler_builder())
  }
}
