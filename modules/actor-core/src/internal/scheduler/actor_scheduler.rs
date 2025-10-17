#![allow(missing_docs)]

use alloc::boxed::Box;
use alloc::vec::Vec;

use async_trait::async_trait;

use super::ready_queue_scheduler::ReadyQueueWorker;
use crate::api::mailbox::mailbox_runtime::MailboxRuntime;
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::supervisor::Supervisor;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::actor::InternalActorRef;
use crate::internal::metrics::MetricsSinkShared;
use crate::internal::scheduler::scheduler_spawn_context::SchedulerSpawnContext;
use crate::internal::scheduler::spawn_error::SpawnError;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use crate::shared::map_system::MapSystemShared;
use crate::shared::receive_timeout::ReceiveTimeoutFactoryShared;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

#[allow(dead_code)]
#[async_trait(?Send)]
pub trait ActorScheduler<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<InternalActorRef<M, R>, SpawnError<M>>;

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>);

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>);

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>);

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>);

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared);

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig);

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>);

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  );

  fn take_escalations(&mut self) -> Vec<FailureInfo>;

  fn actor_count(&self) -> usize;

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>>;

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>>;

  /// Returns a shared worker handle if the scheduler supports ReadyQueue-based execution.
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, R>>> {
    let _ = self;
    None
  }
}
