use core::convert::Infallible;

use crate::runtime::guardian::{AlwaysRestart, GuardianStrategy};
use crate::runtime::mailbox::traits::{ActorRuntime, MailboxRuntime};
use crate::runtime::scheduler::{ReadyQueueWorker, SchedulerBuilder, SchedulerHandle};
use crate::ReceiveTimeoutFactoryShared;
use crate::{default_failure_telemetry, FailureTelemetryShared, TelemetryObservationConfig};
use crate::{Extensions, FailureEventHandler, FailureEventListener, MetricsSinkShared, PriorityEnvelope};
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, QueueError};
use core::marker::PhantomData;

use super::InternalRootContext;

/// Internal configuration used while assembling [`InternalActorSystem`].
pub struct InternalActorSystemSettings<M, R>
where
  M: Element,
  R: ActorRuntime + MailboxRuntime + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Listener invoked for failures reaching the root guardian.
  pub(crate) root_event_listener: Option<FailureEventListener>,
  /// Escalation handler invoked when failures bubble to the root guardian.
  pub(crate) root_escalation_handler: Option<FailureEventHandler>,
  /// Receive-timeout scheduler factory applied to newly spawned actors.
  pub(crate) receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
  /// Metrics sink shared across the actor runtime.
  pub(crate) metrics_sink: Option<MetricsSinkShared>,
  /// Shared registry of actor system extensions.
  pub(crate) extensions: Extensions,
  /// Telemetry invoked when failures reach the root guardian。
  pub(crate) root_failure_telemetry: FailureTelemetryShared,
  /// Observation config applied to telemetry calls.
  pub(crate) root_observation_config: TelemetryObservationConfig,
}

impl<M, R> Default for InternalActorSystemSettings<M, R>
where
  M: Element,
  R: ActorRuntime + MailboxRuntime + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  fn default() -> Self {
    Self {
      root_event_listener: None,
      root_escalation_handler: None,
      receive_timeout_factory: None,
      metrics_sink: None,
      extensions: Extensions::new(),
      root_failure_telemetry: default_failure_telemetry(),
      root_observation_config: TelemetryObservationConfig::new(),
    }
  }
}

pub(crate) struct InternalActorSystem<M, R, Strat = AlwaysRestart>
where
  M: Element + 'static,
  R: ActorRuntime + MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>, {
  pub(super) scheduler: SchedulerHandle<M, R>,
  pub(super) runtime: ArcShared<R>,
  extensions: Extensions,
  metrics_sink: Option<MetricsSinkShared>,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<M, R> InternalActorSystem<M, R, AlwaysRestart>
where
  M: Element,
  R: ActorRuntime + MailboxRuntime + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  pub fn new(actor_runtime: R) -> Self {
    Self::new_with_settings(actor_runtime, InternalActorSystemSettings::default())
  }

  pub fn new_with_settings(actor_runtime: R, settings: InternalActorSystemSettings<M, R>) -> Self {
    let scheduler_builder = ArcShared::new(SchedulerBuilder::<M, R>::ready_queue());
    Self::new_with_settings_and_builder(actor_runtime, &scheduler_builder, settings)
  }

  pub fn new_with_settings_and_builder(
    actor_runtime: R,
    scheduler_builder: &ArcShared<SchedulerBuilder<M, R>>,
    settings: InternalActorSystemSettings<M, R>,
  ) -> Self {
    let InternalActorSystemSettings {
      root_event_listener,
      root_escalation_handler,
      receive_timeout_factory,
      metrics_sink,
      root_failure_telemetry,
      root_observation_config,
      extensions,
    } = settings;
    let actor_runtime_shared = ArcShared::new(actor_runtime);
    let actor_runtime_shared_cloned = actor_runtime_shared.clone();
    let actor_runtime_cloned = actor_runtime_shared.with_ref(|r| r.clone());
    let mut scheduler = scheduler_builder.build(actor_runtime_cloned, extensions.clone());
    scheduler.set_root_event_listener(root_event_listener);
    scheduler.set_root_escalation_handler(root_escalation_handler);
    scheduler.set_root_failure_telemetry(root_failure_telemetry);
    scheduler.set_root_observation_config(root_observation_config);
    scheduler.set_receive_timeout_factory(receive_timeout_factory);
    scheduler.set_metrics_sink(metrics_sink.clone());
    Self {
      scheduler,
      runtime: actor_runtime_shared_cloned,
      extensions,
      metrics_sink,
      _strategy: PhantomData,
    }
  }
}

impl<M, R, Strat> InternalActorSystem<M, R, Strat>
where
  M: Element,
  R: ActorRuntime + MailboxRuntime + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  #[allow(clippy::missing_const_for_fn)]
  pub fn root_context(&mut self) -> InternalRootContext<'_, M, R, Strat> {
    InternalRootContext { system: self }
  }

  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    self.run_until_impl(should_continue).await
  }

  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      self.scheduler.dispatch_next().await?;
    }
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.scheduler.dispatch_next().await
  }

  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.scheduler.drain_ready()
  }

  pub fn run_until_idle<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      let processed = self.drain_ready()?;
      if !processed {
        break;
      }
    }
    Ok(())
  }

  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, R>>> {
    self.scheduler.ready_queue_worker()
  }

  async fn run_until_impl<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.scheduler.dispatch_next().await?;
    }
    Ok(())
  }
}
