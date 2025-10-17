use alloc::boxed::Box;
use alloc::vec::Vec;

use async_trait::async_trait;

use super::ready_queue_scheduler::ReadyQueueWorker;
use crate::api::actor::actor_ref::PriorityActorRef;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::metrics::MetricsSinkShared;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::supervisor::Supervisor;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::scheduler::SchedulerSpawnContext;
use crate::internal::scheduler::SpawnError;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use crate::shared::map_system::MapSystemShared;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Scheduler interface wiring actor spawning, execution, and escalation plumbing.
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait ActorScheduler<M, R>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Spawns a new actor instance and returns its internal reference on success.
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<PriorityActorRef<M, R>, SpawnError<M>>;

  /// Installs a factory used to create receive-timeout drivers for child actors.
  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, R>>);

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
  fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, R>, map_system: MapSystemShared<M>);

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
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, R>>> {
    let _ = self;
    None
  }
}
