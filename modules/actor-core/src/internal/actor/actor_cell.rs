use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::any::TypeId;
use core::cell::RefCell;
use core::cmp::Reverse;
use core::marker::PhantomData;

use crate::api::actor::actor_failure::ActorFailure;
use crate::api::actor::actor_ref::PriorityActorRef;
use crate::api::actor::ActorId;
use crate::api::actor::ActorPath;
use crate::api::extensions::Extensions;
use crate::api::mailbox::Mailbox;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::MailboxHandle;
use crate::api::mailbox::MailboxProducer;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::mailbox::SystemMessage;
use crate::api::messaging::DynMessage;
use crate::api::metrics::MetricsSinkShared;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::supervisor::Supervisor;
use crate::internal::context::{ActorContext, ActorHandlerFn, ChildSpawnSpec};
use crate::internal::guardian::{Guardian, GuardianStrategy};
use crate::internal::mailbox::PriorityMailboxSpawnerHandle;
use crate::internal::scheduler::ReadyQueueHandle;
use crate::internal::scheduler::SpawnError;
use cellex_utils_core_rs::{Element, QueueError};

use crate::api::actor_system::map_system::MapSystemShared;
use crate::api::receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactoryShared};

pub(crate) struct ActorCell<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>, {
  #[cfg_attr(not(feature = "std"), allow(dead_code))]
  actor_id: ActorId,
  map_system: MapSystemShared<M>,
  watchers: Vec<ActorId>,
  actor_path: ActorPath,
  mailbox_factory: MF,
  mailbox_spawner: PriorityMailboxSpawnerHandle<M, MF>,
  mailbox: MF::Mailbox<PriorityEnvelope<M>>,
  sender: MF::Producer<PriorityEnvelope<M>>,
  supervisor: Box<dyn Supervisor<M>>,
  handler: Box<ActorHandlerFn<M, MF>>,
  _strategy: PhantomData<Strat>,
  stopped: bool,
  receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
  receive_timeout_scheduler: Option<RefCell<Box<dyn ReceiveTimeoutScheduler>>>,
  extensions: Extensions,
}

impl<M, MF, Strat> ActorCell<M, MF, Strat>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  Strat: GuardianStrategy<M, MF>,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    actor_id: ActorId,
    map_system: MapSystemShared<M>,
    watchers: Vec<ActorId>,
    actor_path: ActorPath,
    mailbox_factory: MF,
    mailbox_spawner: PriorityMailboxSpawnerHandle<M, MF>,
    mailbox: MF::Mailbox<PriorityEnvelope<M>>,
    sender: MF::Producer<PriorityEnvelope<M>>,
    supervisor: Box<dyn Supervisor<M>>,
    handler: Box<ActorHandlerFn<M, MF>>,
    receive_timeout_factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
    extensions: Extensions,
  ) -> Self {
    let mut cell = Self {
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
      _strategy: PhantomData,
      stopped: false,
      receive_timeout_factory: None,
      receive_timeout_scheduler: None,
      extensions,
    };
    cell.configure_receive_timeout_factory(receive_timeout_factory);
    cell
  }

  pub(crate) fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>)
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<M>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<M>>: Clone, {
    Mailbox::set_metrics_sink(&mut self.mailbox, sink.clone());
    MailboxProducer::set_metrics_sink(&mut self.sender, sink.clone());
    self.mailbox_spawner.set_metrics_sink(sink);
  }

  pub(crate) fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>)
  where
    MF: MailboxFactory + Clone + 'static,
    MF::Queue<PriorityEnvelope<M>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<M>>: Clone, {
    Mailbox::set_scheduler_hook(&mut self.mailbox, hook.clone());
    MailboxProducer::set_scheduler_hook(&mut self.sender, hook);
  }

  pub(in crate::internal) fn configure_receive_timeout_factory(
    &mut self,
    factory: Option<ReceiveTimeoutSchedulerFactoryShared<M, MF>>,
  ) {
    if let Some(cell) = self.receive_timeout_scheduler.as_ref() {
      cell.borrow_mut().cancel();
    }
    self.receive_timeout_scheduler = None;
    self.receive_timeout_factory = factory.clone();
    if let Some(factory_arc) = factory {
      let scheduler = factory_arc.create(self.sender.clone(), self.map_system.clone());
      self.receive_timeout_scheduler = Some(RefCell::new(scheduler));
    }
  }

  pub(super) fn mark_stopped(&mut self, guardian: &mut Guardian<M, MF, Strat>) {
    if self.stopped {
      return;
    }

    self.stopped = true;
    if let Some(cell) = self.receive_timeout_scheduler.as_ref() {
      cell.borrow_mut().cancel();
    }
    self.receive_timeout_scheduler = None;
    self.receive_timeout_factory = None;
    self.mailbox.close();
    let _ = guardian.remove_child(self.actor_id);
    self.watchers.clear();
  }

  pub(super) fn should_mark_stop_for_message() -> bool {
    TypeId::of::<M>() == TypeId::of::<DynMessage>()
  }

  pub(crate) fn has_pending_messages(&self) -> bool {
    !self.mailbox.is_empty()
  }

  fn collect_envelopes(&mut self) -> Result<Vec<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    let mut drained = Vec::new();
    while let Some(envelope) = MailboxHandle::try_dequeue(&self.mailbox)? {
      drained.push(envelope);
    }
    if drained.len() > 1 {
      drained.sort_by_key(|b: &PriorityEnvelope<M>| Reverse(b.priority()));
    }
    Ok(drained)
  }

  fn process_envelopes(
    &mut self,
    envelopes: Vec<PriorityEnvelope<M>>,
    guardian: &mut Guardian<M, MF, Strat>,
    new_children: &mut Vec<ActorCell<M, MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<usize, QueueError<PriorityEnvelope<M>>> {
    let mut processed = 0;
    for envelope in envelopes.into_iter() {
      self.dispatch_envelope(envelope, guardian, new_children, escalations)?;
      processed += 1;
    }
    Ok(processed)
  }

  pub(crate) fn process_pending(
    &mut self,
    guardian: &mut Guardian<M, MF, Strat>,
    new_children: &mut Vec<ActorCell<M, MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<usize, QueueError<PriorityEnvelope<M>>> {
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
    guardian: &mut Guardian<M, MF, Strat>,
    new_children: &mut Vec<ActorCell<M, MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<usize, QueueError<PriorityEnvelope<M>>> {
    if self.stopped {
      return Ok(0);
    }
    let first: PriorityEnvelope<M> = match self.mailbox.recv().await {
      Ok(message) => message,
      Err(QueueError::Disconnected) => return Ok(0),
      Err(err) => return Err(err),
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

  pub(crate) fn is_stopped(&self) -> bool {
    self.stopped
  }

  pub(super) fn dispatch_envelope(
    &mut self,
    envelope: PriorityEnvelope<M>,
    guardian: &mut Guardian<M, MF, Strat>,
    new_children: &mut Vec<ActorCell<M, MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
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
        Ok(handler_result) => self.apply_handler_result(
          handler_result,
          pending_specs,
          should_stop,
          guardian,
          new_children,
          escalations,
        ),
        Err(payload) => {
          let failure = ActorFailure::from_panic_payload(payload.as_ref());
          if let Some(info) = guardian.notify_failure(self.actor_id, failure)? {
            escalations.push(info);
          }
          Ok(())
        }
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
      )
    }
  }

  fn invoke_handler(
    &mut self,
    message: M,
    priority: i8,
    influences_receive_timeout: bool,
    pending_specs: &mut Vec<ChildSpawnSpec<M, MF>>,
  ) -> Result<(), ActorFailure> {
    let receive_timeout = self.receive_timeout_scheduler.as_ref();
    let mut ctx = ActorContext::new(
      &self.mailbox_factory,
      self.mailbox_spawner.clone(),
      &self.sender,
      self.supervisor.as_mut(),
      pending_specs,
      self.map_system.clone(),
      self.actor_path.clone(),
      self.actor_id,
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
    pending_specs: Vec<ChildSpawnSpec<M, MF>>,
    should_stop: bool,
    guardian: &mut Guardian<M, MF, Strat>,
    new_children: &mut Vec<ActorCell<M, MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    match handler_result {
      Ok(()) => {
        for spec in pending_specs.into_iter() {
          self
            .register_child_from_spec(spec, guardian, new_children)
            .map_err(|err| match err {
              SpawnError::Queue(queue_err) => queue_err,
              SpawnError::NameExists(name) => panic!("unexpected named spawn conflict: {name}"),
            })?;
        }
        if should_stop {
          self.mark_stopped(guardian);
        }
        Ok(())
      }
      Err(err) => {
        if let Some(info) = guardian.notify_failure(self.actor_id, err)? {
          escalations.push(info);
        }
        Ok(())
      }
    }
  }

  fn register_child_from_spec(
    &mut self,
    spec: ChildSpawnSpec<M, MF>,
    guardian: &mut Guardian<M, MF, Strat>,
    new_children: &mut Vec<ActorCell<M, MF, Strat>>,
  ) -> Result<(), SpawnError<M>> {
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
    } = spec;

    let control_ref = PriorityActorRef::new(sender.clone());
    let primary_watcher = watchers.first().copied();
    let (actor_id, actor_path) = guardian.register_child_with_naming(
      control_ref,
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
      self.mailbox_factory.clone(),
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      self.receive_timeout_factory.clone(),
      extensions,
    );
    let sink = self.mailbox_spawner.metrics_sink();
    cell.set_metrics_sink(sink);
    new_children.push(cell);
    Ok(())
  }
}
