#![allow(missing_docs)]

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use async_trait::async_trait;

use super::ready_queue_scheduler::ReadyQueueWorker;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::actor::InternalActorRef;
use crate::internal::context::ActorHandlerFn;
use crate::MailboxOptions;
use crate::TelemetryObservationConfig;
use crate::{
  FailureEventHandler, FailureEventListener, FailureInfo, FailureTelemetryShared, MailboxRuntime, MapSystemShared,
  MetricsSinkShared, ReceiveTimeoutFactoryShared, Supervisor,
};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Naming strategy applied when spawning a child actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildNaming {
  /// Automatically assign an incrementing identifier-based name.
  Auto,
  /// Generate a name using the provided prefix followed by a unique suffix.
  WithPrefix(String),
  /// Use the provided name verbatim; fails if the name already exists.
  Explicit(String),
}

impl Default for ChildNaming {
  fn default() -> Self {
    Self::Auto
  }
}

/// Errors that can occur while spawning an actor through the scheduler.
#[derive(Debug)]
pub enum SpawnError<M>
where
  M: Element, {
  /// Underlying mailbox or queue failure.
  Queue(QueueError<PriorityEnvelope<M>>),
  /// Attempted to reuse an existing actor name.
  NameExists(String),
}

impl<M> SpawnError<M>
where
  M: Element,
{
  pub(crate) fn name_exists(name: impl Into<String>) -> Self {
    Self::NameExists(name.into())
  }
}

impl<M> From<QueueError<PriorityEnvelope<M>>> for SpawnError<M>
where
  M: Element,
{
  fn from(value: QueueError<PriorityEnvelope<M>>) -> Self {
    Self::Queue(value)
  }
}

/// Parameters supplied to schedulers when spawning a new actor.
pub struct SchedulerSpawnContext<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  pub mailbox_runtime: R,
  pub mailbox_runtime_shared: ArcShared<R>,
  pub map_system: MapSystemShared<M>,
  pub mailbox_options: MailboxOptions,
  pub handler: Box<ActorHandlerFn<M, R>>,
  /// Naming strategy to apply when registering the child actor.
  pub child_naming: ChildNaming,
}

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
