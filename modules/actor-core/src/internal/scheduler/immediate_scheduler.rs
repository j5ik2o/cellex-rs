#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::api::actor::actor_ref::PriorityActorRef;
use crate::api::extensions::Extensions;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::metrics::MetricsSinkShared;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::supervisor::Supervisor;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::guardian::{AlwaysRestart, GuardianStrategy};
use crate::internal::scheduler::actor_scheduler::ActorScheduler;
use crate::internal::scheduler::ready_queue_scheduler::ReadyQueueScheduler;
use crate::internal::scheduler::SchedulerSpawnContext;
use crate::internal::scheduler::SpawnError;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use crate::shared::map_system::MapSystemShared;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Scheduler wrapper that executes actors immediately using the ReadyQueue scheduler logic.
///
/// This scheduler simply delegates to [`ReadyQueueScheduler`] but exposes a distinct builder entry point.
pub struct ImmediateScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  inner: ReadyQueueScheduler<M, R, Strat>,
}

impl<M, R> ImmediateScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
{
  /// Creates a new immediate scheduler with the default guardian strategy.
  #[must_use]
  pub fn new(mailbox_runtime: R, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::new(mailbox_runtime, extensions),
    }
  }
}

impl<M, R, Strat> ImmediateScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  /// Creates a new immediate scheduler with a custom guardian strategy.
  #[must_use]
  pub fn with_strategy(mailbox_runtime: R, strategy: Strat, extensions: Extensions) -> Self {
    Self {
      inner: ReadyQueueScheduler::with_strategy(mailbox_runtime, strategy, extensions),
    }
  }
}

#[async_trait::async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, R> for ImmediateScheduler<M, R, Strat>
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
    self.inner.dispatch_next().await
  }
}
