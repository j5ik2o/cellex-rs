#[cfg(feature = "unwind-supervision")]
extern crate std;

use alloc::{boxed::Box, collections::VecDeque, vec, vec::Vec};
use core::{cell::RefCell, cmp::Reverse, convert::TryFrom, marker::PhantomData, time::Duration};

use cellex_utils_core_rs::{
  collections::queue::backend::QueueError,
  sync::{shared::Shared, ArcShared},
};

use super::{actor_cell_state::ActorCellState, invoke_result::ActorInvokeOutcome};
use crate::{
  api::{
    actor::{actor_failure::ActorFailure, actor_ref::PriorityActorRef, ActorHandlerFn, ActorId, ActorPath, SpawnError},
    actor_scheduler::{
      ready_queue_coordinator::{InvokeResult, ResumeCondition, SignalKey, SuspendReason},
      ready_queue_scheduler::ReadyQueueHandle,
    },
    extensions::Extensions,
    failure::FailureInfo,
    guardian::{Guardian, GuardianStrategy},
    mailbox::{messages::SystemMessage, Mailbox},
    metrics::{MetricsEvent, MetricsSinkShared, SuspensionClockShared},
    process::{pid::Pid, process_registry::ProcessRegistry},
    receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactoryShared},
    supervision::supervisor::Supervisor,
  },
  internal::{
    actor_context::{ChildSpawnSpec, InternalActorContext},
    mailbox::PriorityMailboxSpawnerHandle,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxConsumer, MailboxFactory, MailboxProducer},
    messaging::{AnyMessage, MapSystemShared},
  },
};

type ActorCellProcessRegistryShared<MF> =
  ArcShared<ProcessRegistry<PriorityActorRef<AnyMessage, MF>, ArcShared<PriorityEnvelope<AnyMessage>>>>;

pub(crate) struct ActorCell<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>, {
  actor_id: ActorId,
  map_system: MapSystemShared<AnyMessage>,
  watchers: Vec<ActorId>,
  actor_path: ActorPath,
  pid: Pid,
  mailbox_factory: MF,
  mailbox_spawner: PriorityMailboxSpawnerHandle<AnyMessage, MF>,
  mailbox: MF::Mailbox<PriorityEnvelope<AnyMessage>>,
  sender: MF::Producer<PriorityEnvelope<AnyMessage>>,
  supervisor: Box<dyn Supervisor<AnyMessage>>,
  handler: Box<ActorHandlerFn<AnyMessage, MF>>,
  _strategy: PhantomData<Strat>,
  stopped: bool,
  state: ActorCellState,
  pending_user_envelopes: VecDeque<PriorityEnvelope<AnyMessage>>,
  suspend_count: u64,
  resume_count: u64,
  suspension_clock: SuspensionClockShared,
  suspend_started_at: Option<u64>,
  last_suspend_nanos: Option<u64>,
  total_suspend_nanos: u128,
  scheduler_hook: Option<ReadyQueueHandle>,
  metrics_sink: Option<MetricsSinkShared>,
  receive_timeout_scheduler_factory_shared_opt: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  receive_timeout_scheduler_opt: Option<RefCell<Box<dyn ReceiveTimeoutScheduler>>>,
  extensions: Extensions,
  process_registry: ActorCellProcessRegistryShared<MF>,
}

impl<MF, Strat> ActorCell<MF, Strat>
where
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<MF>,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    actor_id: ActorId,
    map_system: MapSystemShared<AnyMessage>,
    watchers: Vec<ActorId>,
    actor_path: ActorPath,
    pid: Pid,
    mailbox_factory: MF,
    mailbox_spawner: PriorityMailboxSpawnerHandle<AnyMessage, MF>,
    mailbox: MF::Mailbox<PriorityEnvelope<AnyMessage>>,
    sender: MF::Producer<PriorityEnvelope<AnyMessage>>,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    handler: Box<ActorHandlerFn<AnyMessage, MF>>,
    receive_timeout_scheduler_factory_shared_opt: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
    extensions: Extensions,
    process_registry: ActorCellProcessRegistryShared<MF>,
  ) -> Self {
    let mut cell = Self {
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
      _strategy: PhantomData,
      stopped: false,
      state: ActorCellState::Running,
      pending_user_envelopes: VecDeque::new(),
      suspend_count: 0,
      resume_count: 0,
      suspension_clock: SuspensionClockShared::null(),
      suspend_started_at: None,
      last_suspend_nanos: None,
      total_suspend_nanos: 0,
      scheduler_hook: None,
      metrics_sink: None,
      receive_timeout_scheduler_factory_shared_opt: None,
      receive_timeout_scheduler_opt: None,
      extensions,
      process_registry,
    };
    cell.configure_receive_timeout_scheduler_factory_shared_opt(receive_timeout_scheduler_factory_shared_opt);
    cell
  }

  pub(crate) fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>)
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
    let queue_sink = sink.clone();
    Mailbox::set_metrics_sink(&mut self.mailbox, queue_sink);
    let producer_sink = sink.clone();
    MailboxProducer::set_metrics_sink(&mut self.sender, producer_sink);
    self.mailbox_spawner.set_metrics_sink(sink.clone());
    self.metrics_sink = sink;
  }

  pub(crate) fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>)
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
    self.scheduler_hook = hook.clone();
    Mailbox::set_scheduler_hook(&mut self.mailbox, hook.clone());
    MailboxProducer::set_scheduler_hook(&mut self.sender, hook);
  }

  pub(crate) fn set_suspension_clock(&mut self, clock: SuspensionClockShared)
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
    self.suspension_clock = clock;
    self.suspend_started_at = None;
    self.last_suspend_nanos = None;
    self.total_suspend_nanos = 0;
  }

  pub fn configure_receive_timeout_scheduler_factory_shared_opt(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MF>>,
  ) {
    if let Some(cell) = self.receive_timeout_scheduler_opt.as_ref() {
      cell.borrow_mut().cancel();
    }
    self.receive_timeout_scheduler_opt = None;
    self.receive_timeout_scheduler_factory_shared_opt = factory.clone();
    if let Some(factory_arc) = factory {
      let scheduler = factory_arc.create(self.sender.clone(), self.map_system.clone());
      self.receive_timeout_scheduler_opt = Some(RefCell::new(scheduler));
    }
  }

  pub(super) fn mark_stopped(&mut self, guardian: &mut Guardian<MF, Strat>) {
    if self.stopped {
      return;
    }

    self.stopped = true;
    self.state = ActorCellState::Stopped;
    self.pending_user_envelopes.clear();
    self.scheduler_hook = None;
    self.suspension_clock = SuspensionClockShared::null();
    self.suspend_started_at = None;
    self.last_suspend_nanos = None;
    self.total_suspend_nanos = 0;
    self.suspend_count = 0;
    self.resume_count = 0;
    if let Some(cell) = self.receive_timeout_scheduler_opt.as_ref() {
      cell.borrow_mut().cancel();
    }
    self.receive_timeout_scheduler_opt = None;
    self.receive_timeout_scheduler_factory_shared_opt = None;
    self.mailbox.close();
    self.process_registry.with_ref(|registry| registry.deregister(&self.pid));
    let _ = guardian.remove_child(self.actor_id);
    self.watchers.clear();
  }

  pub(super) const fn should_mark_stop_for_message() -> bool {
    true
  }

  pub(crate) fn has_pending_messages(&self) -> bool {
    !self.mailbox.is_empty() || !self.pending_user_envelopes.is_empty()
  }

  pub(crate) const fn is_suspended(&self) -> bool {
    matches!(self.state, ActorCellState::Suspended)
  }

  fn transition_to_suspended(&mut self) {
    if !self.is_suspended() {
      self.state = ActorCellState::Suspended;
      self.suspend_count = self.suspend_count.saturating_add(1);
      if self.suspend_started_at.is_none() {
        self.suspend_started_at = self.suspension_clock.now();
      }
      self.record_metrics_event(MetricsEvent::MailboxSuspended {
        suspend_count:  self.suspend_count,
        last_duration:  self.last_suspend_duration(),
        total_duration: self.total_suspend_duration(),
      });
    }
  }

  fn transition_to_running(&mut self) {
    if self.is_suspended() {
      self.state = ActorCellState::Running;
      let last_duration_nanos = self
        .suspend_started_at
        .take()
        .and_then(|start| self.suspension_clock.now().map(|end| end.saturating_sub(start)));
      if let Some(nanos) = last_duration_nanos {
        self.last_suspend_nanos = Some(nanos);
        self.total_suspend_nanos = self.total_suspend_nanos.saturating_add(nanos as u128);
      }
      self.resume_count = self.resume_count.saturating_add(1);
      self.record_metrics_event(MetricsEvent::MailboxResumed {
        resume_count:   self.resume_count,
        last_duration:  last_duration_nanos.map(Duration::from_nanos),
        total_duration: self.total_suspend_duration(),
      });
      if !self.pending_user_envelopes.is_empty() {
        if let Some(hook) = &self.scheduler_hook {
          hook.with_ref(|hook| hook.notify_ready());
        }
      }
    }
  }

  fn record_metrics_event(&self, event: MetricsEvent) {
    if let Some(sink) = &self.metrics_sink {
      sink.with_ref(|sink| sink.record(event));
    }
  }

  fn total_suspend_duration(&self) -> Option<Duration> {
    if self.total_suspend_nanos == 0 {
      None
    } else {
      Some(Self::duration_from_nanos(self.total_suspend_nanos))
    }
  }

  fn last_suspend_duration(&self) -> Option<Duration> {
    self.last_suspend_nanos.map(Duration::from_nanos)
  }

  fn duration_from_nanos(nanos: u128) -> Duration {
    if nanos > u64::MAX as u128 {
      Duration::from_nanos(u64::MAX)
    } else {
      Duration::from_nanos(nanos as u64)
    }
  }

  fn make_suspend_outcome(&self) -> InvokeResult {
    InvokeResult::Suspended { reason: SuspendReason::UserDefined, resume_on: self.compute_resume_condition() }
  }

  fn compute_resume_condition(&self) -> ResumeCondition {
    if self.scheduler_hook.is_some() {
      ResumeCondition::ExternalSignal(self.resume_signal_key())
    } else {
      ResumeCondition::WhenCapacityAvailable
    }
  }

  fn resume_signal_key(&self) -> SignalKey {
    let raw = u64::try_from(self.actor_id.0).unwrap_or(u64::MAX);
    SignalKey(raw)
  }

  pub(crate) fn enqueue_system_message(&mut self, message: SystemMessage) {
    let map_system = self.map_system.clone();
    let envelope = PriorityEnvelope::from_system(message).map(move |sys| map_system(sys));
    let _ = self.sender.try_send(envelope);
  }

  fn collect_envelopes(
    &mut self,
  ) -> Result<Vec<PriorityEnvelope<AnyMessage>>, QueueError<PriorityEnvelope<AnyMessage>>> {
    let mut drained = Vec::new();

    if !self.is_suspended() && !self.pending_user_envelopes.is_empty() {
      drained.extend(self.pending_user_envelopes.drain(..));
    }

    while let Some(envelope) = MailboxConsumer::try_dequeue(&self.mailbox)? {
      if self.is_suspended() && envelope.system_message().is_none() {
        self.pending_user_envelopes.push_back(envelope);
        continue;
      }
      drained.push(envelope);
    }
    if drained.len() > 1 {
      drained.sort_by_key(|b: &PriorityEnvelope<AnyMessage>| Reverse(b.priority()));
    }
    Ok(drained)
  }

  fn process_envelopes(
    &mut self,
    envelopes: Vec<PriorityEnvelope<AnyMessage>>,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(usize, ActorInvokeOutcome), QueueError<PriorityEnvelope<AnyMessage>>> {
    let mut outcome = ActorInvokeOutcome::new();
    let mut processed = 0;
    for envelope in envelopes.into_iter() {
      if self.is_suspended() && envelope.system_message().is_none() {
        self.pending_user_envelopes.push_back(envelope);
        continue;
      }
      if outcome.is_set() {
        self.pending_user_envelopes.push_back(envelope);
        continue;
      }
      self.dispatch_envelope(envelope, guardian, new_children, escalations, &mut outcome)?;
      processed += 1;
    }
    Ok((processed, outcome))
  }

  pub(crate) fn process_pending(
    &mut self,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(usize, ActorInvokeOutcome), QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
      return Ok((0, ActorInvokeOutcome::new()));
    }
    let envelopes = self.collect_envelopes()?;
    if envelopes.is_empty() {
      let mut outcome = ActorInvokeOutcome::new();
      if self.is_suspended() {
        outcome.set(self.make_suspend_outcome());
      }
      return Ok((0, outcome));
    }
    self.process_envelopes(envelopes, guardian, new_children, escalations)
  }

  pub(crate) async fn wait_and_process(
    &mut self,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(usize, ActorInvokeOutcome), QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
      return Ok((0, ActorInvokeOutcome::new()));
    }
    let first: PriorityEnvelope<AnyMessage> = match self.mailbox.recv().await {
      | Ok(message) => message,
      | Err(QueueError::Disconnected) => return Ok((0, ActorInvokeOutcome::new())),
      | Err(err) => return Err(err),
    };
    let mut envelopes = vec![first];
    envelopes.extend(self.collect_envelopes()?);
    if envelopes.len() > 1 {
      envelopes.sort_by_key(|b| Reverse(b.priority()));
    }
    self.process_envelopes(envelopes, guardian, new_children, escalations)
  }

  pub(crate) fn signal_clone(&self) -> MF::Signal {
    MailboxConsumer::signal(&self.mailbox)
  }

  pub(crate) const fn is_stopped(&self) -> bool {
    self.stopped
  }

  pub(super) fn dispatch_envelope(
    &mut self,
    envelope: PriorityEnvelope<AnyMessage>,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
    outcome: &mut ActorInvokeOutcome,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
      return Ok(());
    }

    let should_stop =
      matches!(envelope.system_message(), Some(SystemMessage::Stop)) && Self::should_mark_stop_for_message();
    if let Some(SystemMessage::Escalate(failure)) = envelope.system_message().cloned() {
      if let Some(next_failure) = guardian.escalate_failure(failure)? {
        escalations.push(next_failure);
      }
      return Ok(());
    }

    match envelope.system_message() {
      | Some(SystemMessage::Suspend) => {
        self.transition_to_suspended();
        if !outcome.is_set() {
          outcome.set(self.make_suspend_outcome());
        }
      },
      | Some(SystemMessage::Resume) => self.transition_to_running(),
      | _ => {},
    }

    let influences_receive_timeout = envelope.system_message().is_none();
    let (message, priority) = envelope.into_parts();
    self.supervisor.before_handle();
    let mut pending_specs = Vec::new();

    #[cfg(feature = "unwind-supervision")]
    {
      let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        self.invoke_handler(message, priority, influences_receive_timeout, &mut pending_specs)
      }));

      return match result {
        | Ok(handler_result) => self.apply_handler_result(
          handler_result,
          pending_specs,
          should_stop,
          guardian,
          new_children,
          escalations,
          outcome,
        ),
        | Err(payload) => {
          let failure = ActorFailure::from_panic_payload(payload.as_ref());
          if let Some(info) = guardian.notify_failure(self.actor_id, failure)? {
            escalations.push(info);
          }
          Ok(())
        },
      };
    }

    #[cfg(not(feature = "unwind-supervision"))]
    {
      let handler_result = self.invoke_handler(message, priority, influences_receive_timeout, &mut pending_specs);

      self.apply_handler_result(
        handler_result,
        pending_specs,
        should_stop,
        guardian,
        new_children,
        escalations,
        outcome,
      )
    }
  }

  fn invoke_handler(
    &mut self,
    message: AnyMessage,
    priority: i8,
    influences_receive_timeout: bool,
    pending_specs: &mut Vec<ChildSpawnSpec<MF>>,
  ) -> Result<(), ActorFailure> {
    let receive_timeout = self.receive_timeout_scheduler_opt.as_ref();
    let mut ctx = InternalActorContext::new(
      &self.mailbox_factory,
      self.mailbox_spawner.clone(),
      &self.sender,
      self.supervisor.as_mut(),
      pending_specs,
      self.map_system.clone(),
      self.actor_path.clone(),
      self.actor_id,
      self.pid.clone(),
      self.process_registry.clone(),
      &mut self.watchers,
      receive_timeout,
      self.extensions.clone(),
    );
    ctx.enter_priority(priority);
    let handler_result = (self.handler)(&mut ctx, message);
    ctx.notify_receive_timeout_activity(influences_receive_timeout);
    ctx.exit_priority();
    self.supervisor.after_handle();
    handler_result
  }

  #[allow(clippy::too_many_arguments)]
  fn apply_handler_result(
    &mut self,
    handler_result: Result<(), ActorFailure>,
    pending_specs: Vec<ChildSpawnSpec<MF>>,
    should_stop: bool,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
    outcome: &mut ActorInvokeOutcome,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    match handler_result {
      | Ok(()) => {
        for spec in pending_specs.into_iter() {
          self.register_child_from_spec(spec, guardian, new_children).map_err(|err| match err {
            | SpawnError::Queue(queue_err) => queue_err,
            | SpawnError::NameExists(name) => {
              debug_assert!(false, "unexpected named spawn conflict: {name}");
              QueueError::Disconnected
            },
          })?;
        }
        if should_stop {
          self.mark_stopped(guardian);
          if !outcome.is_set() {
            outcome.set(InvokeResult::Stopped);
          }
        }
        Ok(())
      },
      | Err(err) => {
        if let Some(info) = guardian.notify_failure(self.actor_id, err)? {
          escalations.push(info);
        }
        Ok(())
      },
    }
  }

  fn register_child_from_spec(
    &mut self,
    spec: ChildSpawnSpec<MF>,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
  ) -> Result<(), SpawnError<AnyMessage>> {
    let ChildSpawnSpec {
      mailbox,
      sender,
      supervisor,
      handler,
      mailbox_spawner,
      watchers,
      map_system,
      parent_path,
      extensions,
      child_naming,
      pid_slot,
    } = spec;

    let control_ref = PriorityActorRef::new(sender.clone());
    let primary_watcher = watchers.first().copied();
    let (actor_id, actor_path) = guardian.register_child_with_naming(
      control_ref.clone(),
      map_system.clone(),
      primary_watcher,
      &parent_path,
      child_naming,
    )?;
    let control_handle = ArcShared::new(control_ref);
    let pid =
      self.process_registry.with_ref(|registry| registry.register_local(actor_path.clone(), control_handle.clone()));
    {
      let mut slot = pid_slot.write();
      *slot = Some(pid.clone());
    }
    let mut cell = ActorCell::new(
      actor_id,
      map_system,
      watchers,
      actor_path,
      pid,
      self.mailbox_factory.clone(),
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      self.receive_timeout_scheduler_factory_shared_opt.clone(),
      extensions,
      self.process_registry.clone(),
    );
    let sink = self.mailbox_spawner.metrics_sink();
    cell.set_metrics_sink(sink);
    new_children.push(cell);
    Ok(())
  }
}
