#![allow(missing_docs)]

use alloc::boxed::Box;
use alloc::vec::Vec;

use async_trait::async_trait;

use crate::api::actor::MailboxHandleFactoryStub;
use crate::runtime::context::{ActorHandlerFn, InternalActorRef};
use crate::MailboxOptions;
use crate::TelemetryObservationConfig;
use crate::{
  Extensions, FailureEventHandler, FailureEventListener, FailureInfo, FailureTelemetryShared, MailboxRuntime,
  MapSystemShared, MetricsSinkShared, PriorityEnvelope, ReceiveTimeoutFactoryShared, Supervisor,
};
use cellex_utils_core_rs::sync::{ArcShared, Shared, SharedBound};
use cellex_utils_core_rs::{Element, QueueError};

use super::priority_scheduler::ReadyQueueWorker;

pub(crate) type SchedulerHandle<M, R> = Box<dyn ActorScheduler<M, R>>;
#[cfg(target_has_atomic = "ptr")]
type FactoryFn<M, R> = dyn Fn(R, Extensions) -> SchedulerHandle<M, R> + Send + Sync + 'static;
#[cfg(not(target_has_atomic = "ptr"))]
type FactoryFn<M, R> = dyn Fn(R, Extensions) -> SchedulerHandle<M, R> + 'static;

/// Parameters supplied to schedulers when spawning a new actor.
pub struct SchedulerSpawnContext<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  pub runtime: R,
  pub mailbox_handle_factory_stub: MailboxHandleFactoryStub<R>,
  pub map_system: MapSystemShared<M>,
  pub mailbox_options: MailboxOptions,
  pub handler: Box<ActorHandlerFn<M, R>>,
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
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>>;

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

#[derive(Clone)]
pub struct SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  factory: ArcShared<FactoryFn<M, R>>,
}

impl<M, R> SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  #[cfg(any(test, feature = "test-support"))]
  #[allow(dead_code)]
  #[must_use]
  pub fn immediate() -> Self {
    use super::immediate_scheduler::ImmediateScheduler;

    Self::new(|runtime, extensions| Box::new(ImmediateScheduler::new(runtime, extensions)))
  }

  pub fn new<F>(factory: F) -> Self
  where
    F: Fn(R, Extensions) -> SchedulerHandle<M, R> + SharedBound + 'static, {
    let shared = ArcShared::new(factory);
    Self {
      factory: shared.into_dyn(|inner| inner as &FactoryFn<M, R>),
    }
  }

  pub fn build(&self, runtime: R, extensions: Extensions) -> SchedulerHandle<M, R> {
    self.factory.with_ref(|factory| (factory)(runtime, extensions))
  }
}
