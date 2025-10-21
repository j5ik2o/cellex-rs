use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use cellex_utils_core_rs::{sync::ArcShared, QueueError};

use super::ready_queue_scheduler::ReadyQueueWorker;
use crate::api::{
  actor::{actor_ref::PriorityActorRef, SpawnError},
  actor_scheduler::ActorSchedulerSpawnContext,
  actor_system::map_system::MapSystemShared,
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  messaging::AnyMessage,
  metrics::MetricsSinkShared,
  receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
  supervision::{
    escalation::FailureEventHandler, supervisor::Supervisor,
    telemetry::TelemetryObservationConfig,
  },
};
use crate::api::failure::failure_event_stream::FailureEventListener;
use crate::api::failure::failure_telemetry::FailureTelemetryShared;
use crate::api::failure::FailureInfo;

/// Scheduler interface wiring actor spawning, execution, and escalation plumbing.
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait ActorScheduler<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  /// Spawns a new actor instance and returns its internal reference on success.
  ///
  /// # Errors
  /// Returns [`SpawnError`] when the scheduler fails to initialise the actor.
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<PriorityActorRef<AnyMessage, MF>, SpawnError<AnyMessage>>;

  /// Installs a factory used to create receive-timeout drivers for child actors.
  fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
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
  fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  );

  /// Registers a callback invoked when escalations occur during execution.
  fn on_escalation(&mut self, handler: EscalationHandler);

  /// Drains and returns buffered escalations captured since the last poll.
  fn take_escalations(&mut self) -> Vec<FailureInfo>;

  /// Returns the number of actor references currently tracked by the scheduler.
  fn actor_count(&self) -> usize;

  /// Drains ready queues and reports whether additional work remains.
  ///
  /// # Errors
  /// Returns [`QueueError`] when draining ready queues fails.
  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>>;

  /// Dispatches the next scheduled message, awaiting asynchronous readiness.
  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>;

  /// Returns a shared worker handle if the scheduler supports ReadyQueue-based execution.
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MF>>> {
    let _ = self;
    None
  }
}
type EscalationHandler = Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static>;
