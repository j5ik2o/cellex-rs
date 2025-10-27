use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::{collections::queue::backend::QueueError, sync::ArcShared};
use futures::future::LocalBoxFuture;
use spin::Mutex;

use super::{common::ReadyQueueSchedulerCore, ready_queue_state::ReadyQueueState};
use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, SpawnError},
    actor_scheduler::{ready_queue_coordinator::ReadyQueueCoordinator, ActorSchedulerSpawnContext},
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{FailureTelemetryObservationConfig, FailureTelemetryShared},
      FailureInfo,
    },
    guardian::GuardianStrategy,
    metrics::MetricsSinkShared,
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::supervisor::Supervisor,
  },
  internal::actor::ActorCell,
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::{AnyMessage, MapSystemShared},
    supervision::FailureEventHandler,
  },
};

pub(crate) struct ReadyQueueContext<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  pub(crate) core:  ReadyQueueSchedulerCore<MF, Strat>,
  pub(crate) state: ArcShared<Mutex<ReadyQueueState>>,
}

impl<MF, Strat> ReadyQueueContext<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  pub(crate) const fn actor_count(&self) -> usize {
    self.core.actor_count()
  }

  pub(crate) fn actor_mut(&mut self, index: usize) -> Option<&mut ActorCell<MF, Strat>> {
    self.core.actor_mut(index)
  }

  pub(crate) fn actor_has_pending(&self, index: usize) -> bool {
    self.core.actor_has_pending(index)
  }

  pub(crate) fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<(PriorityActorRef<AnyMessage, MF>, usize), SpawnError<AnyMessage>> {
    let actor_ref = self.core.spawn_actor(supervisor, context)?;
    let index = self.core.actor_count().saturating_sub(1);
    Ok((actor_ref, index))
  }

  pub(crate) fn enqueue_ready(&self, index: usize) {
    let mut state = self.state.lock();
    let _ = state.enqueue_if_idle(index);
  }

  pub(crate) fn dequeue_ready(&self) -> Option<usize> {
    let mut state = self.state.lock();
    let index = state.pop_front()?;
    state.mark_running(index);
    Some(index)
  }

  pub(crate) fn mark_idle(&self, index: usize, has_pending: bool) {
    let mut state = self.state.lock();
    state.mark_idle(index, has_pending);
  }

  pub(crate) fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.core.drain_ready()
  }

  pub(crate) fn process_actor_pending(
    &mut self,
    index: usize,
  ) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.core.process_actor_pending(index)
  }

  pub(crate) fn wait_for_any_signal_future(&self) -> Option<LocalBoxFuture<'static, usize>> {
    self.core.wait_for_any_signal_future()
  }

  pub(crate) fn process_ready_once(&mut self) -> Result<Option<bool>, QueueError<PriorityEnvelope<AnyMessage>>> {
    if let Some(index) = self.dequeue_ready() {
      let processed = self.core.process_actor_pending(index)?;
      let has_pending = self.actor_has_pending(index);
      self.mark_idle(index, has_pending);
      return Ok(Some(processed));
    }

    if self.core.drain_ready()? {
      return Ok(Some(true));
    }

    Ok(None)
  }

  pub(crate) fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static, {
    self.core.on_escalation(handler)
  }

  pub(crate) fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.core.take_escalations()
  }

  pub(crate) fn set_receive_timeout_scheduler_factory_shared(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  ) {
    self.core.set_receive_timeout_scheduler_factory_shared_opt(factory)
  }

  pub(crate) fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.core.set_metrics_sink(sink)
  }

  pub(crate) fn set_ready_queue_coordinator(&mut self, coordinator: Option<Box<dyn ReadyQueueCoordinator>>) {
    self.core.set_ready_queue_coordinator(coordinator);
  }

  pub(crate) fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) {
    self.core.set_parent_guardian(control_ref, map_system)
  }

  pub(crate) fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.core.set_root_event_listener(listener)
  }

  pub(crate) fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    self.core.set_root_escalation_handler(handler)
  }

  pub(crate) fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.core.set_root_failure_telemetry(telemetry)
  }

  pub(crate) fn set_root_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
    self.core.set_root_observation_config(config)
  }
}
