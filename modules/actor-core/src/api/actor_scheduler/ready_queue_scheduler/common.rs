use alloc::{boxed::Box, vec, vec::Vec};
use core::{convert::Infallible, marker::PhantomData};

use cellex_utils_core_rs::{Element, QueueError};
use futures::{
  future::{select_all, LocalBoxFuture},
  FutureExt,
};

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ActorId, ActorPath, SpawnError},
    actor_scheduler::ActorSchedulerSpawnContext,
    actor_system::map_system::MapSystemShared,
    extensions::Extensions,
    failure_telemetry::FailureTelemetryShared,
    mailbox::{Mailbox, MailboxFactory, MailboxProducer, MailboxSignal, PriorityEnvelope, SystemMessage},
    metrics::{MetricsEvent, MetricsSinkShared},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::{
      escalation::EscalationSink, failure::FailureInfo, supervisor::Supervisor, telemetry::TelemetryObservationConfig,
    },
  },
  internal::{
    actor::ActorCell,
    guardian::{AlwaysRestart, Guardian, GuardianStrategy},
    mailbox::PriorityMailboxSpawnerHandle,
    supervision::CompositeEscalationSink,
  },
};

/// Simple scheduler implementation assuming priority mailboxes.
pub(crate) struct ReadyQueueSchedulerCore<M, MF, Strat = AlwaysRestart>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>, {
  pub(crate) guardian:     Guardian<M, MF, Strat>,
  actors:                  Vec<ActorCell<M, MF, Strat>>,
  escalations:             Vec<FailureInfo>,
  escalation_sink:         CompositeEscalationSink<M, MF>,
  receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
  metrics_sink:            Option<MetricsSinkShared>,
  extensions:              Extensions,
  _strategy:               PhantomData<Strat>,
}

#[allow(dead_code)]
impl<M, MF> ReadyQueueSchedulerCore<M, MF, AlwaysRestart>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
{
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self::with_strategy(mailbox_factory, AlwaysRestart, extensions)
  }

  pub fn with_strategy<Strat>(
    _mailbox_factory: MF,
    strategy: Strat,
    extensions: Extensions,
  ) -> ReadyQueueSchedulerCore<M, MF, Strat>
  where
    Strat: GuardianStrategy<M, MF>, {
    ReadyQueueSchedulerCore {
      guardian: Guardian::new(strategy),
      actors: Vec::new(),
      escalations: Vec::new(),
      escalation_sink: CompositeEscalationSink::default(),
      receive_timeout_factory: None,
      metrics_sink: None,
      extensions,
      _strategy: PhantomData,
    }
  }
}

#[allow(dead_code)]
impl<M, MF, Strat> ReadyQueueSchedulerCore<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  pub fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: ActorSchedulerSpawnContext<M, MF>,
  ) -> Result<PriorityActorRef<M, MF>, SpawnError<M>> {
    let ActorSchedulerSpawnContext {
      mailbox_factory,
      mailbox_factory_shared,
      map_system,
      mailbox_options,
      handler,
      child_naming,
    } = context;
    let mut mailbox_spawner = PriorityMailboxSpawnerHandle::new(mailbox_factory_shared);
    mailbox_spawner.set_metrics_sink(self.metrics_sink.clone());
    let (mut mailbox, mut sender) = mailbox_spawner.spawn_mailbox(mailbox_options);
    mailbox.set_metrics_sink(self.metrics_sink.clone());
    sender.set_metrics_sink(self.metrics_sink.clone());
    let actor_sender = sender.clone();
    let control_ref = PriorityActorRef::new(actor_sender.clone());
    let watchers = vec![ActorId::ROOT];
    let primary_watcher = watchers.first().copied();
    let parent_path = ActorPath::new();
    let (actor_id, actor_path) = self.guardian.register_child_with_naming(
      control_ref.clone(),
      map_system.clone(),
      primary_watcher,
      &parent_path,
      child_naming,
    )?;
    let mut cell = ActorCell::new(
      actor_id,
      map_system,
      watchers,
      actor_path,
      mailbox_factory,
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      self.receive_timeout_factory.clone(),
      self.extensions.clone(),
    );
    cell.set_metrics_sink(self.metrics_sink.clone());
    self.actors.push(cell);
    self.record_metric(MetricsEvent::ActorRegistered);
    Ok(control_ref)
  }

  /// Legacy sync API. Internally uses the same path as `dispatch_next`,
  /// but `run_until` / `dispatch_next` is recommended for new code.
  #[deprecated(since = "3.1.0", note = "Use dispatch_next / run_until instead")]
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    #[cfg(feature = "std")]
    {
      use core::sync::atomic::{AtomicBool, Ordering};
      static WARNED: AtomicBool = AtomicBool::new(false);
      if !WARNED.swap(true, Ordering::Relaxed) {
        tracing::warn!(
          "ReadyQueueScheduler::dispatch_all is deprecated. Consider using dispatch_next / run_until instead."
        );
      }
    }
    let _ = self.drain_ready_cycle()?;
    Ok(())
  }

  /// Helper that repeats `dispatch_next` as long as the condition holds.
  /// Allows simple construction of wait loops that can be controlled from the runtime side.
  pub async fn run_until<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.dispatch_next().await?;
    }
    Ok(())
  }

  /// Runs the scheduler as a resident async task. Can be used like
  /// `tokio::spawn(async move { scheduler.run_forever().await })`.
  /// Stops on error or task cancellation.
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      self.dispatch_next().await?;
    }
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    loop {
      if self.drain_ready_cycle()? {
        return Ok(());
      }

      let Some(wait_future) = self.wait_for_any_signal_future() else {
        return Ok(());
      };
      let index = wait_future.await;

      if self.process_waiting_actor(index).await? {
        return Ok(());
      }
    }
  }

  pub fn actor_count(&self) -> usize {
    self.actors.len()
  }

  pub fn actor_mut(&mut self, index: usize) -> Option<&mut ActorCell<M, MF, Strat>> {
    self.actors.get_mut(index)
  }

  pub fn actor_has_pending(&self, index: usize) -> bool {
    self.actors.get(index).map(|cell| cell.has_pending_messages()).unwrap_or(false)
  }

  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    core::mem::take(&mut self.escalations)
  }

  /// Processes one cycle of messages in the Ready queue. Returns true if processing occurred.
  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.drain_ready_cycle()
  }

  pub fn set_receive_timeout_scheduler_factory_shared_opt(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
  ) {
    self.receive_timeout_factory = factory.clone();
    for actor in &mut self.actors {
      actor.configure_receive_timeout_factory(factory.clone());
    }
  }

  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink.clone();
    for actor in &mut self.actors {
      actor.set_metrics_sink(sink.clone());
    }
  }

  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    self.escalation_sink.set_custom_handler(handler);
  }

  pub fn set_parent_guardian(&mut self, control_ref: PriorityActorRef<M, MF>, map_system: MapSystemShared<M>) {
    self.escalation_sink.set_parent_guardian(control_ref, map_system);
  }

  pub fn set_root_escalation_handler(
    &mut self,
    handler: Option<crate::api::supervision::escalation::FailureEventHandler>,
  ) {
    self.escalation_sink.set_root_handler(handler);
  }

  pub fn set_root_event_listener(
    &mut self,
    listener: Option<crate::api::supervision::escalation::FailureEventListener>,
  ) {
    self.escalation_sink.set_root_listener(listener);
  }

  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.escalation_sink.set_root_telemetry(telemetry);
  }

  pub fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.escalation_sink.set_root_observation_config(config);
  }

  fn handle_escalations(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if self.escalations.is_empty() {
      return Ok(false);
    }

    let pending = core::mem::take(&mut self.escalations);
    let mut remaining = Vec::new();
    let mut handled = false;
    for info in pending.into_iter() {
      let handled_locally = self.forward_to_local_parent(&info);
      match self.escalation_sink.handle(info, handled_locally) {
        | Ok(()) => handled = true,
        | Err(unhandled) => remaining.push(unhandled),
      }
    }
    self.escalations = remaining;
    Ok(handled)
  }

  pub(crate) fn wait_for_any_signal_future(&self) -> Option<LocalBoxFuture<'static, usize>> {
    if self.actors.is_empty() {
      return None;
    }

    let mut waiters = Vec::with_capacity(self.actors.len());
    for (idx, cell) in self.actors.iter().enumerate() {
      let signal = cell.signal_clone();
      waiters.push(
        async move {
          signal.wait().await;
          idx
        }
        .boxed_local(),
      );
    }

    Some(Box::pin(async move {
      let (idx, _, _) = select_all(waiters).await;
      idx
    }))
  }

  fn drain_ready_cycle(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    let mut new_children = Vec::new();
    let len = self.actors.len();
    let mut processed_any = false;
    for idx in 0..len {
      let cell = &mut self.actors[idx];
      let processed = cell.process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
      if processed > 0 {
        self.record_messages_dequeued(processed);
        processed_any = true;
      }
    }
    self.finish_cycle(new_children, processed_any)
  }

  async fn process_waiting_actor(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if index >= self.actors.len() {
      return Ok(false);
    }

    let mut new_children = Vec::new();
    let processed_count =
      self.actors[index].wait_and_process(&mut self.guardian, &mut new_children, &mut self.escalations).await?;
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    self.finish_cycle(new_children, processed_count > 0)
  }

  pub fn process_actor_pending(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if index >= self.actors.len() {
      return Ok(false);
    }

    let mut new_children = Vec::new();
    let processed_count =
      self.actors[index].process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    self.finish_cycle(new_children, processed_count > 0)
  }

  fn finish_cycle(
    &mut self,
    new_children: Vec<ActorCell<M, MF, Strat>>,
    processed_any: bool,
  ) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if !new_children.is_empty() {
      let added = new_children.len();
      self.actors.extend(new_children);
      self.record_repeated(MetricsEvent::ActorRegistered, added);
    }

    let handled = self.handle_escalations()?;
    let removed = self.prune_stopped();
    Ok(processed_any || handled || removed)
  }

  fn forward_to_local_parent(&self, info: &FailureInfo) -> bool {
    if let Some(parent_info) = info.escalate_to_parent() {
      if parent_info.path.is_empty() {
        return false;
      }

      if let Some((parent_ref, map_system)) = self.guardian.child_route(parent_info.actor) {
        #[allow(clippy::redundant_closure)]
        let envelope =
          PriorityEnvelope::from_system(SystemMessage::Escalate(parent_info)).map(|sys| (&*map_system)(sys));
        if parent_ref.sender().try_send(envelope).is_ok() {
          return true;
        }
      }
    }

    false
  }

  fn prune_stopped(&mut self) -> bool {
    let before = self.actors.len();
    self.actors.retain(|cell| !cell.is_stopped());
    let removed = before.saturating_sub(self.actors.len());
    if removed > 0 {
      self.record_repeated(MetricsEvent::ActorDeregistered, removed);
      return true;
    }
    false
  }

  fn record_metric(&self, event: MetricsEvent) {
    self.record_repeated(event, 1);
  }

  fn record_messages_dequeued(&self, count: usize) {
    self.record_repeated(MetricsEvent::MailboxDequeued, count);
  }

  fn record_repeated(&self, event: MetricsEvent, count: usize) {
    if count == 0 {
      return;
    }
    if let Some(sink) = &self.metrics_sink {
      sink.with_ref(|sink| {
        for _ in 0..count {
          sink.record(event);
        }
      });
    }
  }
}
