#![cfg(feature = "new-runtime")]

//! `NewActorRuntimeBundle` 実装: Tokio ホスト環境向けバンドル。

use std::sync::Arc;
use std::vec::Vec;

use async_trait::async_trait;
use cellex_actor_core_rs::{ActorScheduler, AlwaysRestart, DynMessage, Extensions, FailureEventHandler, FailureEventListener, FailureInfo, GuardianStrategy, InternalActorRef, MailboxHandleFactoryStub, MailboxRuntime, MapSystemShared, MetricsSinkShared, NewActorRuntimeBundle, NewMailboxHandleFactory, PriorityEnvelope, PriorityScheduler, ReceiveTimeoutFactoryShared, SchedulerBuilder, SchedulerSpawnContext, SharedSchedulerBuilder, Supervisor};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};
use tokio::task::yield_now;

use crate::receive_timeout::TokioReceiveTimeoutSchedulerFactory;
use crate::tokio_mailbox::TokioMailboxRuntime;

/// Tokio ランタイム向け `ActorScheduler`。
pub struct TokioSchedulerNew<M, Strat = AlwaysRestart>
where
  M: Element,
  Strat: GuardianStrategy<M, TokioMailboxRuntime>,
{
  inner: PriorityScheduler<M, TokioMailboxRuntime, Strat>,
}

impl<M> TokioSchedulerNew<M>
where
  M: Element,
{
  /// Constructs a Tokio-aware scheduler wrapper around [`PriorityScheduler`].
  pub fn new(runtime: TokioMailboxRuntime, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::new(runtime, extensions),
    }
  }
}

impl<M, Strat> TokioSchedulerNew<M, Strat>
where
  M: Element,
  Strat: GuardianStrategy<M, TokioMailboxRuntime>,
{
  /// Constructs a Tokio-aware scheduler with a custom guardian strategy.
  pub fn with_strategy(runtime: TokioMailboxRuntime, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: PriorityScheduler::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[async_trait(?Send)]
impl<M, Strat> ActorScheduler<M, TokioMailboxRuntime> for TokioSchedulerNew<M, Strat>
where
  M: Element,
  Strat: GuardianStrategy<M, TokioMailboxRuntime> + Send + Sync,
  <TokioMailboxRuntime as MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
  <TokioMailboxRuntime as MailboxRuntime>::Signal: Clone,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, TokioMailboxRuntime>,
  ) -> Result<InternalActorRef<M, TokioMailboxRuntime>, QueueError<PriorityEnvelope<M>>> {
    self.inner.spawn_actor(supervisor, context)
  }

  fn set_receive_timeout_factory(
    &mut self,
    factory: Option<ReceiveTimeoutFactoryShared<M, TokioMailboxRuntime>>,
  ) {
    self.inner.set_receive_timeout_factory(factory);
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.inner.set_root_event_listener(listener);
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    self.inner.set_root_escalation_handler(handler);
  }

  fn set_parent_guardian(
    &mut self,
    control_ref: InternalActorRef<M, TokioMailboxRuntime>,
    map_system: MapSystemShared<M>,
  ) {
    self.inner.set_parent_guardian(control_ref, map_system);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    self.inner.on_escalation(handler);
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

fn tokio_scheduler_builder() -> SchedulerBuilder<DynMessage, TokioMailboxRuntime> {
  SchedulerBuilder::new(|runtime, extensions| Box::new(TokioSchedulerNew::new(runtime, extensions)))
}

/// Tokio ホスト向けの新ランタイムバンドル。
#[derive(Clone)]
pub struct HostTokioBundle {
  mailbox_factory: ArcShared<MailboxHandleFactoryStub<TokioMailboxRuntime>>,
  scheduler_builder: ArcShared<SharedSchedulerBuilder<TokioMailboxRuntime>>,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, TokioMailboxRuntime>>,
  metrics_sink: Option<MetricsSinkShared>,
  root_event_listener: Option<FailureEventListener>,
  root_escalation_handler: Option<FailureEventHandler>,
  extensions: Extensions,
}

impl HostTokioBundle {
  /// 既定構成のバンドルを生成する。
  #[must_use]
  pub fn new() -> Self {
    let runtime = TokioMailboxRuntime;
    let mailbox_stub = MailboxHandleFactoryStub::from_runtime(runtime);
    let mailbox_factory = ArcShared::new(mailbox_stub);

    let scheduler = SharedSchedulerBuilder::from_builder(tokio_scheduler_builder());
    let scheduler_builder = ArcShared::new(scheduler);

    let receive_timeout_factory = Some(ReceiveTimeoutFactoryShared::new(TokioReceiveTimeoutSchedulerFactory::new()));

    Self {
      mailbox_factory,
      scheduler_builder,
      receive_timeout_factory,
      metrics_sink: None,
      root_event_listener: None,
      root_escalation_handler: None,
      extensions: Extensions::new(),
    }
  }

  /// メトリクスシンクを設定する。
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// ルートイベントリスナーを設定する。
  #[must_use]
  pub fn with_root_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.root_event_listener = listener;
    self
  }

  /// ルートエスカレーションハンドラを設定する。
  #[must_use]
  pub fn with_root_escalation_handler(mut self, handler: Option<FailureEventHandler>) -> Self {
    self.root_escalation_handler = handler;
    self
  }

  /// ReceiveTimeout ファクトリを上書きする。
  #[must_use]
  pub fn with_receive_timeout_factory(
    mut self,
    factory: Option<ReceiveTimeoutFactoryShared<DynMessage, TokioMailboxRuntime>>,
  ) -> Self {
    self.receive_timeout_factory = factory;
    self
  }

  /// 拡張を直接操作する。
  pub fn extensions_mut(&mut self) -> &mut Extensions {
    &mut self.extensions
  }
}

impl Default for HostTokioBundle {
  fn default() -> Self {
    Self::new()
  }
}

impl NewActorRuntimeBundle for HostTokioBundle {
  type MailboxRuntime = TokioMailboxRuntime;
  type SchedulerBuilder = SharedSchedulerBuilder<TokioMailboxRuntime>;

  fn mailbox_handle_factory(&self) -> ArcShared<dyn NewMailboxHandleFactory<Self::MailboxRuntime>> {
    let stub: ArcShared<_> = self.mailbox_factory.clone();
    let arc: Arc<MailboxHandleFactoryStub<Self::MailboxRuntime>> = ArcShared::into_arc(stub);
    ArcShared::from_arc(arc as Arc<dyn NewMailboxHandleFactory<Self::MailboxRuntime>>)
  }

  fn scheduler_builder(&self) -> ArcShared<Self::SchedulerBuilder> {
    self.scheduler_builder.clone()
  }

  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self::MailboxRuntime>> {
    self.receive_timeout_factory.clone()
  }

  fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  fn root_event_listener(&self) -> Option<FailureEventListener> {
    self.root_event_listener.clone()
  }

  fn root_escalation_handler(&self) -> Option<FailureEventHandler> {
    self.root_escalation_handler.clone()
  }

  fn extensions(&self) -> &Extensions {
    &self.extensions
  }
}
