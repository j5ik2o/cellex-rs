#![cfg(feature = "embassy_executor")]

use alloc::boxed::Box;
use alloc::vec::Vec;

use cellex_actor_core_rs::{
  ActorScheduler, AlwaysRestart, Extensions, FailureEventHandler, FailureEventListener, FailureInfo, GuardianStrategy,
  InternalActorRef, MailboxRuntime, MapSystemShared, MetricsSinkShared, PriorityEnvelope, ReadyQueueScheduler,
  ReceiveTimeoutFactoryShared, RuntimeEnv, SchedulerBuilder, SchedulerSpawnContext, Supervisor,
};
use cellex_utils_embedded_rs::Element;
use embassy_executor::Spawner;
use embassy_futures::yield_now;

use crate::receive_timeout::EmbassyReceiveTimeoutSchedulerFactory;

/// Embassy 用スケジューラ。
///
/// ReadyQueue ベースの [`cellex_actor_core_rs::ReadyQueueScheduler`] をラップし、`embassy_futures::yield_now` による協調切り替えを提供する。
pub struct EmbassyScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, RuntimeEnv<R>>, {
  inner: ReadyQueueScheduler<M, RuntimeEnv<R>, Strat>,
}

impl<M, R> EmbassyScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  /// 既定の GuardianStrategy (`AlwaysRestart`) を用いた構成を作成する。
  pub fn new(runtime: RuntimeEnv<R>, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::new(runtime, extensions),
    }
  }
}

impl<M, R, Strat> EmbassyScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, RuntimeEnv<R>>,
{
  /// 任意の GuardianStrategy を適用した構成を作成する。
  pub fn with_strategy(runtime: RuntimeEnv<R>, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, RuntimeEnv<R>> for EmbassyScheduler<M, R, Strat>
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
  ) -> Result<InternalActorRef<M, RuntimeEnv<R>>, cellex_utils_embedded_rs::QueueError<PriorityEnvelope<M>>> {
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

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    ReadyQueueScheduler::set_metrics_sink(&mut self.inner, sink);
  }

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, RuntimeEnv<R>>, map_system: MapSystemShared<M>) {
    ReadyQueueScheduler::set_parent_guardian(&mut self.inner, control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<
      dyn FnMut(&FailureInfo) -> Result<(), cellex_utils_embedded_rs::QueueError<PriorityEnvelope<M>>> + 'static,
    >,
  ) {
    ReadyQueueScheduler::on_escalation(&mut self.inner, handler);
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
pub fn embassy_scheduler_builder<M, R>() -> SchedulerBuilder<M, RuntimeEnv<R>>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  SchedulerBuilder::new(|runtime, extensions| {
    Box::new(EmbassyScheduler::<M, R, AlwaysRestart>::new(runtime, extensions))
  })
}

/// `ActorRuntimeBundle` に Embassy スケジューラを組み込むための拡張トレイト。
pub trait ActorRuntimeBundleEmbassyExt<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<cellex_actor_core_rs::DynMessage>>: Clone,
  R::Signal: Clone, {
  /// スケジューラを Embassy 実装へ差し替える。
  fn with_embassy_scheduler(self, spawner: &'static Spawner) -> RuntimeEnv<R>;
}

impl<R> ActorRuntimeBundleEmbassyExt<R> for RuntimeEnv<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<cellex_actor_core_rs::DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn with_embassy_scheduler(self, spawner: &'static Spawner) -> RuntimeEnv<R> {
    let bundle = self.with_scheduler_builder(embassy_scheduler_builder());
    if bundle.receive_timeout_factory().is_some() {
      bundle
    } else {
      bundle.with_receive_timeout_factory(ReceiveTimeoutFactoryShared::new(
        EmbassyReceiveTimeoutSchedulerFactory::<R>::new(spawner),
      ))
    }
  }
}
