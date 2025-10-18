use alloc::{boxed::Box, vec::Vec};

use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};
use futures::future::LocalBoxFuture;
use spin::Mutex;

use super::{common::ReadyQueueSchedulerCore, ready_queue_state::ReadyQueueState};
use crate::{
  api::{
    actor::actor_ref::PriorityActorRef,
    actor_system::map_system::MapSystemShared,
    failure_telemetry::FailureTelemetryShared,
    mailbox::{MailboxFactory, PriorityEnvelope},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::{
      escalation::{FailureEventHandler, FailureEventListener},
      failure::FailureInfo,
      supervisor::Supervisor,
      telemetry::TelemetryObservationConfig,
    },
  },
  internal::{
    actor::ActorCell,
    guardian::GuardianStrategy,
    scheduler::{spawn_error::SpawnError, SchedulerSpawnContext},
  },
};

pub(crate) struct ReadyQueueContext<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>, {
  pub(crate) core:  ReadyQueueSchedulerCore<M, MF, Strat>,
  pub(crate) state: ArcShared<Mutex<ReadyQueueState>>,
}

impl<M, MF, Strat> ReadyQueueContext<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  pub(super) fn actor_count(&self) -> usize {
    self.core.actor_count()
  }

  pub(super) fn actor_mut(&mut self, index: usize) -> Option<&mut ActorCell<M, MF, Strat>> {
    self.core.actor_mut(index)
  }

  pub(super) fn actor_has_pending(&self, index: usize) -> bool {
    self.core.actor_has_pending(index)
  }

  pub(super) fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, MF>,
  ) -> Result<(PriorityActorRef<M, MF>, usize), SpawnError<M>> {
    let actor_ref = self.core.spawn_actor(supervisor, context)?;
    let index = self.core.actor_count().saturating_sub(1);
    Ok((actor_ref, index))
  }

  pub(super) fn enqueue_ready(&self, index: usize) {
    let mut state = self.state.lock();
    let _ = state.enqueue_if_idle(index);
  }

  pub(super) fn dequeue_ready(&self) -> Option<usize> {
    let mut state = self.state.lock();
    let index = state.queue.pop_front()?;
    state.queued[index] = false;
    state.mark_running(index);
    Some(index)
  }

  pub(super) fn mark_idle(&self, index: usize, has_pending: bool) {
    let mut state = self.state.lock();
    state.mark_idle(index, has_pending);
  }

  pub(super) fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.core.drain_ready()
  }

  pub(super) fn process_actor_pending(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.core.process_actor_pending(index)
  }

  pub(super) fn wait_for_any_signal_future(&self) -> Option<LocalBoxFuture<'static, usize>> {
    self.core.wait_for_any_signal_future()
  }

  pub(super) fn process_ready_once(&mut self) -> Result<Option<bool>, QueueError<PriorityEnvelope<M>>> {
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

  pub(super) fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    self.core.on_escalation(handler)
  }

  pub(super) fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.core.take_escalations()
  }

  pub(super) fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>) {
    self.core.set_receive_timeout_factory(factory)
  }

  pub(super) fn set_metrics_sink(&mut self, sink: Option<crate::api::metrics::MetricsSinkShared>) {
    self.core.set_metrics_sink(sink)
  }

  pub(super) fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, MF>, map_system: MapSystemShared<M>) {
    self.core.set_parent_guardian(control_ref, map_system)
  }

  pub(super) fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.core.set_root_event_listener(listener)
  }

  pub(super) fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    self.core.set_root_escalation_handler(handler)
  }

  pub(super) fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.core.set_root_failure_telemetry(telemetry)
  }

  pub(super) fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.core.set_root_observation_config(config)
  }
}
