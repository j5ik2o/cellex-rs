#[cfg(feature = "unwind-supervision")]
extern crate std;

use alloc::{boxed::Box, vec, vec::Vec};
use core::{cell::RefCell, cmp::Reverse, marker::PhantomData};

use cellex_utils_core_rs::{collections::queue::QueueError, sync::ArcShared, Shared};

use crate::{
  api::{
    actor::{actor_failure::ActorFailure, actor_ref::PriorityActorRef, ActorHandlerFn, ActorId, ActorPath, SpawnError},
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    extensions::Extensions,
    failure::FailureInfo,
    guardian::{Guardian, GuardianStrategy},
    mailbox::{messages::SystemMessage, Mailbox, MailboxFactory, MailboxHandle, MailboxProducer},
    metrics::MetricsSinkShared,
    process::{pid::Pid, process_registry::ProcessRegistry},
    receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactoryShared},
    supervision::supervisor::Supervisor,
  },
  internal::{
    actor_context::{ChildSpawnSpec, InternalActorContext},
    mailbox::PriorityMailboxSpawnerHandle,
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
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
    Mailbox::set_metrics_sink(&mut self.mailbox, sink.clone());
    MailboxProducer::set_metrics_sink(&mut self.sender, sink.clone());
    self.mailbox_spawner.set_metrics_sink(sink);
  }

  pub(crate) fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>)
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
    Mailbox::set_scheduler_hook(&mut self.mailbox, hook.clone());
    MailboxProducer::set_scheduler_hook(&mut self.sender, hook);
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
    !self.mailbox.is_empty()
  }

  fn collect_envelopes(
    &mut self,
  ) -> Result<Vec<PriorityEnvelope<AnyMessage>>, QueueError<PriorityEnvelope<AnyMessage>>> {
    let mut drained = Vec::new();
    while let Some(envelope) = MailboxHandle::try_dequeue(&self.mailbox)? {
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
  ) -> Result<usize, QueueError<PriorityEnvelope<AnyMessage>>> {
    let mut processed = 0;
    for envelope in envelopes.into_iter() {
      self.dispatch_envelope(envelope, guardian, new_children, escalations)?;
      processed += 1;
    }
    Ok(processed)
  }

  pub(crate) fn process_pending(
    &mut self,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<usize, QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
      return Ok(0);
    }
    let envelopes = self.collect_envelopes()?;
    if envelopes.is_empty() {
      return Ok(0);
    }
    self.process_envelopes(envelopes, guardian, new_children, escalations)
  }

  pub(crate) async fn wait_and_process(
    &mut self,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<usize, QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
      return Ok(0);
    }
    let first: PriorityEnvelope<AnyMessage> = match self.mailbox.recv().await {
      | Ok(message) => message,
      | Err(QueueError::Disconnected) => return Ok(0),
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
    MailboxHandle::signal(&self.mailbox)
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
        | Ok(handler_result) => {
          self.apply_handler_result(handler_result, pending_specs, should_stop, guardian, new_children, escalations)
        },
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

      self.apply_handler_result(handler_result, pending_specs, should_stop, guardian, new_children, escalations)
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
