use alloc::{boxed::Box, vec, vec::Vec};
use core::{convert::Infallible, marker::PhantomData};

use cellex_utils_core_rs::{sync::ArcShared, QueueError, Shared};
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
    failure_event_stream::FailureEventListener,
    failure_telemetry::FailureTelemetryShared,
    guardian::{AlwaysRestart, Guardian, GuardianStrategy},
    mailbox::{
      messages::{PriorityEnvelope, SystemMessage},
      Mailbox, MailboxFactory, MailboxProducer, MailboxSignal,
    },
    messaging::AnyMessage,
    metrics::{MetricsEvent, MetricsSinkShared},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::{
      escalation::EscalationSink, failure::FailureInfo, supervisor::Supervisor, telemetry::TelemetryObservationConfig,
    },
  },
  internal::{actor::ActorCell, mailbox::PriorityMailboxSpawnerHandle, supervision::CompositeEscalationSink},
};

/// Simple scheduler implementation assuming priority mailboxes.
pub(crate) struct ReadyQueueSchedulerCore<MF, Strat = AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  pub(crate) guardian: Guardian<MF, Strat>,
  actors: Vec<ActorCell<MF, Strat>>,
  escalations: Vec<FailureInfo>,
  escalation_sink: CompositeEscalationSink<MF>,
  receive_timeout_scheduler_shared_opt: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  metrics_sink_opt: Option<MetricsSinkShared>,
  extensions: Extensions,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<MF> ReadyQueueSchedulerCore<MF, AlwaysRestart>
where
  MF: MailboxFactory + Clone + 'static,
{
  pub fn new(mailbox_factory: MF, extensions: Extensions) -> Self {
    Self::with_strategy(mailbox_factory, AlwaysRestart, extensions)
  }

  pub fn with_strategy<Strat>(
    _mailbox_factory: MF,
    strategy: Strat,
    extensions: Extensions,
  ) -> ReadyQueueSchedulerCore<MF, Strat>
  where
    Strat: GuardianStrategy<MF>, {
    ReadyQueueSchedulerCore {
      guardian: Guardian::new(strategy),
      actors: Vec::new(),
      escalations: Vec::new(),
      escalation_sink: CompositeEscalationSink::default(),
      receive_timeout_scheduler_shared_opt: None,
      metrics_sink_opt: None,
      extensions,
      _strategy: PhantomData,
    }
  }
}

#[allow(dead_code)]
impl<MF, Strat> ReadyQueueSchedulerCore<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  pub fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    context: ActorSchedulerSpawnContext<MF>,
  ) -> Result<PriorityActorRef<AnyMessage, MF>, SpawnError<AnyMessage>> {
    let ActorSchedulerSpawnContext {
      mailbox_factory,
      mailbox_factory_shared,
      map_system,
      mailbox_options,
      handler,
      child_naming,
      process_registry,
      actor_pid_slot,
    } = context;
    let mut mailbox_spawner = PriorityMailboxSpawnerHandle::new(mailbox_factory_shared);
    mailbox_spawner.set_metrics_sink(self.metrics_sink_opt.clone());
    let (mut mailbox, mut sender) = mailbox_spawner.spawn_mailbox(mailbox_options);
    mailbox.set_metrics_sink(self.metrics_sink_opt.clone());
    sender.set_metrics_sink(self.metrics_sink_opt.clone());
    let control_ref = PriorityActorRef::new(sender.clone());
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
    let control_handle = ArcShared::new(control_ref.clone());
    let pid = process_registry.with_ref(|registry| registry.register_local(actor_path.clone(), control_handle.clone()));
    {
      let mut slot = actor_pid_slot.write();
      *slot = Some(pid.clone());
    }

    let mut cell = ActorCell::new(
      actor_id,
      map_system,
      watchers,
      actor_path,
      pid,
      mailbox_factory,
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      self.receive_timeout_scheduler_shared_opt.clone(),
      self.extensions.clone(),
      process_registry,
    );
    cell.set_metrics_sink(self.metrics_sink_opt.clone());
    self.actors.push(cell);
    self.record_metric(MetricsEvent::ActorRegistered);
    Ok(control_ref)
  }

  /// Legacy sync API. Internally uses the same path as `dispatch_next`,
  /// but `run_until` / `dispatch_next` is recommended for new code.
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue operations fail.
  #[deprecated(since = "3.1.0", note = "Use dispatch_next / run_until instead")]
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
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
  ///
  /// # Errors
  /// Returns [`QueueError`] when dispatching an actor fails.
  pub async fn run_until<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
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
  ///
  /// # Errors
  /// Returns [`QueueError`] when dispatching an actor fails.
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>> {
    loop {
      self.dispatch_next().await?;
    }
  }

  /// Dispatches the next ready actor if available, waiting otherwise.
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue processing fails.
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
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

  pub const fn actor_count(&self) -> usize {
    self.actors.len()
  }

  pub fn actor_mut(&mut self, index: usize) -> Option<&mut ActorCell<MF, Strat>> {
    self.actors.get_mut(index)
  }

  pub fn actor_has_pending(&self, index: usize) -> bool {
    self.actors.get(index).map(|cell| cell.has_pending_messages()).unwrap_or(false)
  }

  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    core::mem::take(&mut self.escalations)
  }

  /// Processes one cycle of messages in the Ready queue. Returns true if processing occurred.
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue operations fail.
  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.drain_ready_cycle()
  }

  #[allow(clippy::needless_pass_by_value)]
  pub fn set_receive_timeout_scheduler_factory_shared_opt(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  ) {
    self.receive_timeout_scheduler_shared_opt = factory.clone();
    for actor in &mut self.actors {
      actor.configure_receive_timeout_scheduler_factory_shared_opt(factory.clone());
    }
  }

  #[allow(clippy::needless_pass_by_value)]
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink_opt = sink.clone();
    for actor in &mut self.actors {
      actor.set_metrics_sink(sink.clone());
    }
  }

  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static, {
    self.escalation_sink.set_custom_handler(handler);
  }

  pub fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) {
    self.escalation_sink.set_parent_guardian(control_ref, map_system);
  }

  pub fn set_root_escalation_handler(
    &mut self,
    handler: Option<crate::api::supervision::escalation::FailureEventHandler>,
  ) {
    self.escalation_sink.set_root_handler(handler);
  }

  pub fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.escalation_sink.set_root_listener(listener);
  }

  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.escalation_sink.set_root_telemetry(telemetry);
  }

  pub fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.escalation_sink.set_root_observation_config(config);
  }

  fn handle_escalations(&mut self) -> bool {
    if self.escalations.is_empty() {
      return false;
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
    handled
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

  fn drain_ready_cycle(&mut self) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
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
    Ok(self.finish_cycle(new_children, processed_any))
  }

  async fn process_waiting_actor(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    if index >= self.actors.len() {
      return Ok(false);
    }

    let mut new_children = Vec::new();
    let processed_count =
      self.actors[index].wait_and_process(&mut self.guardian, &mut new_children, &mut self.escalations).await?;
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    Ok(self.finish_cycle(new_children, processed_count > 0))
  }

  pub fn process_actor_pending(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<AnyMessage>>> {
    if index >= self.actors.len() {
      return Ok(false);
    }

    let mut new_children = Vec::new();
    let processed_count =
      self.actors[index].process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    Ok(self.finish_cycle(new_children, processed_count > 0))
  }

  fn finish_cycle(&mut self, new_children: Vec<ActorCell<MF, Strat>>, processed_any: bool) -> bool {
    if !new_children.is_empty() {
      let added = new_children.len();
      self.actors.extend(new_children);
      self.record_repeated(MetricsEvent::ActorRegistered, added);
    }

    let handled = self.handle_escalations();
    let removed = self.prune_stopped();
    processed_any || handled || removed
  }

  fn forward_to_local_parent(&self, info: &FailureInfo) -> bool {
    if let Some(parent_info) = info.escalate_to_parent() {
      if parent_info.path.is_empty() {
        return false;
      }

      if let Some((parent_ref, map_system)) = self.guardian.child_route(parent_info.actor) {
        #[allow(clippy::redundant_clone)]
        let map_clone = map_system.clone();
        #[allow(clippy::redundant_closure)]
        let envelope =
          PriorityEnvelope::from_system(SystemMessage::Escalate(parent_info)).map(move |sys| map_clone(sys));
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
    if let Some(sink) = &self.metrics_sink_opt {
      sink.with_ref(|sink| {
        for _ in 0..count {
          sink.record(event);
        }
      });
    }
  }
}
