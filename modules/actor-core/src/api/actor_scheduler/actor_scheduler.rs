use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};

use super::ready_queue_scheduler::ReadyQueueWorker;
use crate::api::{
  actor::{actor_ref::PriorityActorRef, SpawnError},
  actor_scheduler::ActorSchedulerSpawnContext,
  actor_system::map_system::MapSystemShared,
  failure_telemetry::FailureTelemetryShared,
  mailbox::{MailboxFactory, PriorityEnvelope},
  metrics::MetricsSinkShared,
  receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
  supervision::{
    escalation::{FailureEventHandler, FailureEventListener},
    failure::FailureInfo,
    supervisor::Supervisor,
    telemetry::TelemetryObservationConfig,
  },
};

/// Scheduler interface wiring actor spawning, execution, and escalation plumbing.
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait ActorScheduler<M, MF>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  /// Spawns a new actor instance and returns its internal reference on success.
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: ActorSchedulerSpawnContext<M, MF>,
  ) -> Result<PriorityActorRef<M, MF>, SpawnError<M>>;

  /// Installs a factory used to create receive-timeout drivers for child actors.
  fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
  );

  /// Registers a metrics sink that records scheduler queue statistics.
  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>);

  /// Sets the listener receiving root-level failure events.
  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>);

  /// Sets the handler responsible for propagating root escalations.
  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>);

  /// Provides shared telemetry infrastructure for failure reporting.
  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared);

  /// Configures observation parameters used by failure telemetry.
  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig);

  /// Wires the parent guardian reference used for supervising spawned actors.
  fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, MF>, map_system: MapSystemShared<M>);

  /// Registers a callback invoked when escalations occur during execution.
  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  );

  /// Drains and returns buffered escalations captured since the last poll.
  fn take_escalations(&mut self) -> Vec<FailureInfo>;

  /// Returns the number of actor references currently tracked by the scheduler.
  fn actor_count(&self) -> usize;

  /// Drains ready queues and reports whether additional work remains.
  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>>;

  /// Dispatches the next scheduled message, awaiting asynchronous readiness.
  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>>;

  /// Returns a shared worker handle if the scheduler supports ReadyQueue-based execution.
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, MF>>> {
    let _ = self;
    None
  }
}
