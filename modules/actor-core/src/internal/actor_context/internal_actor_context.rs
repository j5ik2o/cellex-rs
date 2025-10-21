use alloc::{boxed::Box, vec, vec::Vec};
use core::{cell::RefCell, time::Duration};

use cellex_utils_core_rs::{sync::ArcShared, QueueError};
use spin::RwLock;

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ActorHandlerFn, ActorId, ActorPath, ChildNaming},
    actor_system::map_system::MapSystemShared,
    extensions::{Extension, ExtensionId, Extensions},
    mailbox::{messages::PriorityEnvelope, MailboxFactory, MailboxOptions, MailboxProducer},
    process::{pid::Pid, process_registry::ProcessRegistry},
    receive_timeout::ReceiveTimeoutScheduler,
    supervision::supervisor::Supervisor,
  },
  internal::{actor::InternalProps, actor_context::ChildSpawnSpec, mailbox::PriorityMailboxSpawnerHandle},
  shared::messaging::AnyMessage,
};

type ActorProcessRegistryShared<MF> =
  ArcShared<ProcessRegistry<PriorityActorRef<AnyMessage, MF>, ArcShared<PriorityEnvelope<AnyMessage>>>>;

/// Context used by the runtime to interact with the currently running actor and its children.
pub struct InternalActorContext<'a, MF>
where
  MF: MailboxFactory + Clone, {
  mailbox_factory:  &'a MF,
  mailbox_spawner:  PriorityMailboxSpawnerHandle<AnyMessage, MF>,
  sender:           &'a MF::Producer<PriorityEnvelope<AnyMessage>>,
  #[allow(dead_code)]
  supervisor:       &'a mut dyn Supervisor<AnyMessage>,
  #[allow(dead_code)]
  pending_spawns:   &'a mut Vec<ChildSpawnSpec<MF>>,
  #[allow(dead_code)]
  map_system:       MapSystemShared<AnyMessage>,
  actor_path:       ActorPath,
  actor_id:         ActorId,
  pid:              Pid,
  process_registry: ActorProcessRegistryShared<MF>,
  watchers:         &'a mut Vec<ActorId>,
  current_priority: Option<i8>,
  receive_timeout:  Option<&'a RefCell<Box<dyn ReceiveTimeoutScheduler>>>,
  extensions:       Extensions,
}

impl<'a, MF> InternalActorContext<'a, MF>
where
  MF: MailboxFactory + Clone,
{
  #[allow(clippy::too_many_arguments)]
  /// Creates a new internal context for the specified actor.
  pub(crate) fn new(
    mailbox_factory: &'a MF,
    mailbox_spawner: PriorityMailboxSpawnerHandle<AnyMessage, MF>,
    sender: &'a MF::Producer<PriorityEnvelope<AnyMessage>>,
    supervisor: &'a mut dyn Supervisor<AnyMessage>,
    pending_spawns: &'a mut Vec<ChildSpawnSpec<MF>>,
    map_system: MapSystemShared<AnyMessage>,
    actor_path: ActorPath,
    actor_id: ActorId,
    pid: Pid,
    process_registry: ActorProcessRegistryShared<MF>,
    watchers: &'a mut Vec<ActorId>,
    receive_timeout: Option<&'a RefCell<Box<dyn ReceiveTimeoutScheduler>>>,
    extensions: Extensions,
  ) -> Self {
    Self {
      mailbox_factory,
      mailbox_spawner,
      sender,
      supervisor,
      pending_spawns,
      map_system,
      actor_path,
      actor_id,
      pid,
      process_registry,
      watchers,
      current_priority: None,
      receive_timeout,
      extensions,
    }
  }

  /// Returns a clone of the extension registry.
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Applies the provided closure to the extension identified by `id`.
  pub fn extension<E, F, T>(&self, id: ExtensionId, f: F) -> Option<T>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> T, {
    self.extensions.with::<E, _, _>(id, f)
  }

  #[allow(clippy::missing_const_for_fn)]
  /// Returns the mailbox factory used to create new actor mailboxes.
  pub fn mailbox_factory(&self) -> &MF {
    self.mailbox_factory
  }

  #[allow(dead_code)]
  /// Returns the mailbox spawner handle used to provision child mailboxes.
  pub(crate) const fn mailbox_spawner(&self) -> &PriorityMailboxSpawnerHandle<AnyMessage, MF> {
    &self.mailbox_spawner
  }

  #[allow(dead_code)]
  /// Returns a mutable reference to the supervisor overseeing the actor.
  pub(crate) fn supervisor(&mut self) -> &mut dyn Supervisor<AnyMessage> {
    self.supervisor
  }

  /// Returns the numeric identifier of the actor.
  pub const fn actor_id(&self) -> ActorId {
    self.actor_id
  }

  /// Returns the logical actor path.
  pub const fn actor_path(&self) -> &ActorPath {
    &self.actor_path
  }

  /// Returns the list of watchers currently observing the actor.
  pub const fn watchers(&self) -> &[ActorId] {
    self.watchers.as_slice()
  }

  /// Returns the actor's process identifier.
  pub const fn pid(&self) -> &Pid {
    &self.pid
  }

  /// Returns a shared handle to the process registry.
  pub fn process_registry(&self) -> ActorProcessRegistryShared<MF> {
    self.process_registry.clone()
  }

  /// Registers a watcher so that it receives termination notifications.
  pub fn register_watcher(&mut self, watcher: ActorId) {
    if !self.watchers.contains(&watcher) {
      self.watchers.push(watcher);
    }
  }

  /// Unregisters a watcher, stopping termination notifications.
  pub fn unregister_watcher(&mut self, watcher: ActorId) {
    if let Some(index) = self.watchers.iter().position(|w| *w == watcher) {
      self.watchers.swap_remove(index);
    }
  }

  /// Returns an actor reference to the actor itself.
  pub(crate) fn self_ref(&self) -> PriorityActorRef<AnyMessage, MF>
  where
    MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
    PriorityActorRef::new(self.sender.clone())
  }

  #[allow(dead_code)]
  fn enqueue_spawn(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    options: MailboxOptions,
    map_system: MapSystemShared<AnyMessage>,
    handler: Box<ActorHandlerFn<AnyMessage, MF>>,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
  ) -> PriorityActorRef<AnyMessage, MF> {
    let (mailbox, sender) = self.mailbox_spawner.spawn_mailbox(options);
    let actor_ref = PriorityActorRef::new(sender.clone());
    let watchers = vec![self.actor_id];
    self.pending_spawns.push(ChildSpawnSpec {
      mailbox,
      sender,
      supervisor,
      handler,
      mailbox_spawner: self.mailbox_spawner.clone(),
      watchers,
      map_system,
      parent_path: self.actor_path.clone(),
      extensions: self.extensions.clone(),
      child_naming: ChildNaming::Auto,
      pid_slot,
    });
    actor_ref
  }

  /// Queues the creation of a child actor based on prepared props.
  pub(crate) fn spawn_child_from_props(
    &mut self,
    supervisor: Box<dyn Supervisor<AnyMessage>>,
    props: InternalProps<MF>,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
  ) -> PriorityActorRef<AnyMessage, MF>
  where
    MF: MailboxFactory + Clone + 'static, {
    let InternalProps { options, map_system, handler } = props;
    self.enqueue_spawn(supervisor, options, map_system, handler, pid_slot)
  }

  /// Returns the priority currently being processed by the actor, if any.
  pub const fn current_priority(&self) -> Option<i8> {
    self.current_priority
  }

  /// Sends a user message to the actor with an explicit priority.
  pub fn send_to_self_with_priority(
    &self,
    message: AnyMessage,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.sender.try_send(PriorityEnvelope::new(message, priority))
  }

  /// Sends a control message to the actor.
  pub fn send_control_to_self(
    &self,
    message: AnyMessage,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.sender.try_send(PriorityEnvelope::control(message, priority))
  }

  /// Sends a prepared priority envelope to the actor.
  pub fn send_envelope_to_self(
    &self,
    envelope: PriorityEnvelope<AnyMessage>,
  ) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.sender.try_send(envelope)
  }

  #[allow(clippy::missing_const_for_fn)]
  /// Records that a priority is currently being processed.
  pub(crate) fn enter_priority(&mut self, priority: i8) {
    self.current_priority = Some(priority);
  }

  #[allow(clippy::missing_const_for_fn)]
  /// Clears the priority currently being processed.
  pub(crate) fn exit_priority(&mut self) {
    self.current_priority = None;
  }

  /// Returns `true` when a receive-timeout scheduler is installed.
  pub const fn has_receive_timeout_scheduler(&self) -> bool {
    self.receive_timeout.is_some()
  }

  /// Configures the receive-timeout duration for the actor.
  pub fn set_receive_timeout(&mut self, duration: Duration) -> bool {
    if let Some(cell) = self.receive_timeout {
      cell.borrow_mut().set(duration);
      true
    } else {
      false
    }
  }

  /// Cancels a previously configured receive-timeout.
  pub fn cancel_receive_timeout(&mut self) -> bool {
    if let Some(cell) = self.receive_timeout {
      cell.borrow_mut().cancel();
      true
    } else {
      false
    }
  }

  /// Notifies the receive-timeout scheduler about message activity.
  pub(crate) fn notify_receive_timeout_activity(&mut self, influence: bool) {
    if !influence {
      return;
    }

    if let Some(cell) = self.receive_timeout {
      cell.borrow_mut().notify_activity();
    }
  }
}
