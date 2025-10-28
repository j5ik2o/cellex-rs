use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
use core::{
  convert::{Infallible, TryFrom},
  marker::PhantomData,
  time::Duration,
};

use cellex_utils_core_rs::{
  collections::queue::backend::QueueError,
  sync::{shared::Shared, ArcShared},
};
use futures::{
  future::{select_all, LocalBoxFuture},
  FutureExt,
};

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ActorId, ActorPath, SpawnError},
    actor_scheduler::{
      ready_queue_coordinator::{InvokeResult, MailboxIndex, ReadyQueueCoordinator, ResumeCondition, SignalKey},
      ActorSchedulerSpawnContext,
    },
    extensions::Extensions,
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{FailureTelemetryObservationConfig, FailureTelemetryShared},
      FailureInfo,
    },
    guardian::{AlwaysRestart, Guardian, GuardianStrategy},
    mailbox::{messages::SystemMessage, Mailbox},
    metrics::{MetricsEvent, MetricsSinkShared, SuspensionClockShared},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::supervisor::Supervisor,
  },
  internal::{
    actor::{ActorCell, ActorInvokeOutcome},
    mailbox::PriorityMailboxSpawnerHandle,
    supervision::CompositeEscalationSink,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory, MailboxProducer, MailboxSignal},
    messaging::{AnyMessage, MapSystemShared},
    supervision::EscalationSink,
  },
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
  suspension_clock: SuspensionClockShared,
  extensions: Extensions,
  _strategy: PhantomData<Strat>,
  ready_coordinator: Option<Box<dyn ReadyQueueCoordinator>>,
  suspended_conditions: BTreeMap<usize, ResumeCondition>,
  suspended_signals: BTreeMap<SignalKey, usize>,
  suspended_deadlines: BTreeMap<u64, Vec<usize>>,
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
      suspension_clock: SuspensionClockShared::null(),
      extensions,
      _strategy: PhantomData,
      ready_coordinator: None,
      suspended_conditions: BTreeMap::new(),
      suspended_signals: BTreeMap::new(),
      suspended_deadlines: BTreeMap::new(),
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
    cell.set_suspension_clock(self.suspension_clock.clone());
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

  pub fn actor_is_suspended(&self, index: usize) -> bool {
    self.actors.get(index).map(|cell| cell.is_suspended()).unwrap_or(false)
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

  pub fn set_ready_queue_coordinator(&mut self, coordinator: Option<Box<dyn ReadyQueueCoordinator>>) {
    self.ready_coordinator = coordinator;
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

  #[allow(clippy::needless_pass_by_value)]
  pub fn set_suspension_clock(&mut self, clock: SuspensionClockShared) {
    self.suspension_clock = clock.clone();
    for actor in &mut self.actors {
      actor.set_suspension_clock(clock.clone());
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

  pub fn set_root_escalation_handler(&mut self, handler: Option<crate::shared::supervision::FailureEventHandler>) {
    self.escalation_sink.set_root_handler(handler);
  }

  pub fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.escalation_sink.set_root_listener(listener);
  }

  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.escalation_sink.set_root_telemetry(telemetry);
  }

  pub fn set_root_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
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
    self.process_deadlines();
    let mut new_children = Vec::new();
    let len = self.actors.len();
    let mut processed_any = false;
    for idx in 0..len {
      let cell = &mut self.actors[idx];
      let (processed, outcome) = cell.process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
      self.handle_invoke_outcome(idx, processed, outcome);
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
    let (processed_count, outcome) =
      self.actors[index].wait_and_process(&mut self.guardian, &mut new_children, &mut self.escalations).await?;
    self.handle_invoke_outcome(index, processed_count, outcome);
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
    let (processed_count, outcome) =
      self.actors[index].process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
    self.handle_invoke_outcome(index, processed_count, outcome);
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    Ok(self.finish_cycle(new_children, processed_count > 0))
  }

  fn handle_invoke_outcome(&mut self, index: usize, processed: usize, outcome: ActorInvokeOutcome) {
    let Some(result) = self.compose_invoke_result(index, processed, outcome) else {
      return;
    };
    self.update_suspend_registry(index, &result);
    if let Some(coordinator) = self.ready_coordinator.as_mut() {
      let slot = u32::try_from(index).unwrap_or(u32::MAX);
      let mailbox_index = MailboxIndex::new(slot, 0);
      coordinator.handle_invoke_result(mailbox_index, result);
    }
  }

  fn update_suspend_registry(&mut self, index: usize, result: &InvokeResult) {
    match result {
      | InvokeResult::Suspended { resume_on, .. } => {
        self.suspended_conditions.insert(index, resume_on.clone());
        match resume_on {
          | ResumeCondition::ExternalSignal(key) => {
            self.suspended_signals.insert(*key, index);
          },
          | ResumeCondition::After(duration) => {
            self.register_deadline(index, *duration);
          },
          | ResumeCondition::WhenCapacityAvailable => {
            self.resume_actor(index);
          },
        }
      },
      | _ => {
        self.clear_suspend_state(index);
      },
    }
  }

  fn register_deadline(&mut self, index: usize, duration: Duration) {
    let Some(now) = self.suspension_clock.now() else {
      self.resume_actor(index);
      return;
    };
    let nanos = duration.as_nanos();
    let delta = if nanos > u64::MAX as u128 { u64::MAX } else { nanos as u64 };
    let due = now.saturating_add(delta);
    self.suspended_deadlines.entry(due).or_default().push(index);
  }

  fn process_deadlines(&mut self) {
    let Some(now) = self.suspension_clock.now() else {
      return;
    };
    let mut due_entries: Vec<(u64, Vec<usize>)> = Vec::new();
    while let Some((&due, _)) = self.suspended_deadlines.iter().next() {
      if due > now {
        break;
      }
      let indices = self.suspended_deadlines.remove(&due).unwrap_or_default();
      due_entries.push((due, indices));
    }
    for (_, indices) in due_entries {
      for index in indices {
        if matches!(self.suspended_conditions.get(&index), Some(ResumeCondition::After(_))) {
          self.resume_actor(index);
        }
      }
    }
  }

  fn clear_suspend_state(&mut self, index: usize) {
    self.suspended_conditions.remove(&index);
    self.remove_signal_mapping(index);
    self.remove_deadline_entry(index);
  }

  fn resume_actor(&mut self, index: usize) {
    if !self.suspended_conditions.contains_key(&index) {
      return;
    }
    self.clear_suspend_state(index);
    if let Some(actor) = self.actors.get_mut(index) {
      actor.enqueue_system_message(SystemMessage::Resume);
    }
    if let Some(coordinator) = self.ready_coordinator.as_mut() {
      let slot = u32::try_from(index).unwrap_or(u32::MAX);
      coordinator.register_ready(MailboxIndex::new(slot, 0));
    }
  }

  fn remove_signal_mapping(&mut self, index: usize) {
    if let Some((&key, _)) = self.suspended_signals.iter().find(|(_, mapped)| **mapped == index) {
      self.suspended_signals.remove(&key);
    }
  }

  fn remove_deadline_entry(&mut self, index: usize) {
    let deadlines: Vec<u64> = self
      .suspended_deadlines
      .iter()
      .filter_map(|(due, indices)| if indices.iter().any(|i| *i == index) { Some(*due) } else { None })
      .collect();
    for due in deadlines {
      if let Some(mut indices) = self.suspended_deadlines.remove(&due) {
        indices.retain(|i| *i != index);
        if !indices.is_empty() {
          self.suspended_deadlines.insert(due, indices);
        }
      }
    }
  }

  pub fn notify_resume_signal(&mut self, key: SignalKey) -> Option<usize> {
    let index = self.suspended_signals.remove(&key)?;
    self.resume_actor(index);
    Some(index)
  }

  #[allow(dead_code)]
  pub(crate) fn inject_invoke_result_for_testing(&mut self, index: usize, result: InvokeResult) {
    self.update_suspend_registry(index, &result);
  }

  fn compose_invoke_result(
    &mut self,
    index: usize,
    processed: usize,
    outcome: ActorInvokeOutcome,
  ) -> Option<InvokeResult> {
    if let Some(result) = outcome.into_result() {
      return Some(result);
    }
    if processed == 0 {
      return None;
    }
    let result = if self.actors.get(index).map(|cell| cell.is_stopped()).unwrap_or(false) {
      InvokeResult::Stopped
    } else {
      let ready_hint = self.actors.get(index).map(|cell| cell.has_pending_messages()).unwrap_or(false);
      InvokeResult::Completed { ready_hint }
    };
    Some(result)
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
        if parent_ref.try_send_envelope_mailbox(envelope).is_ok() {
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
