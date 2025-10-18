#![allow(missing_docs)]

use alloc::{boxed::Box, vec, vec::Vec};
use core::{cell::RefCell, marker::PhantomData, time::Duration};

use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};
use spin::RwLock;

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, ActorHandlerFn, ActorId, ActorPath, ChildNaming},
    actor_system::map_system::MapSystemShared,
    extensions::{Extension, ExtensionId, Extensions},
    mailbox::{MailboxFactory, MailboxOptions, MailboxProducer, PriorityEnvelope},
    process::{pid::Pid, process_registry::ProcessRegistry},
    receive_timeout::ReceiveTimeoutScheduler,
    supervision::supervisor::Supervisor,
  },
  internal::{actor::InternalProps, context::ChildSpawnSpec, mailbox::PriorityMailboxSpawnerHandle},
};

/// Context for actors to operate on themselves and child actors.
pub struct ActorContext<'a, M, MF, Sup>
where
  M: Element,
  MF: MailboxFactory + Clone,
  Sup: Supervisor<M> + ?Sized, {
  mailbox_factory:  &'a MF,
  mailbox_spawner:  PriorityMailboxSpawnerHandle<M, MF>,
  sender:           &'a MF::Producer<PriorityEnvelope<M>>,
  supervisor:       &'a mut Sup,
  #[allow(dead_code)]
  pending_spawns:   &'a mut Vec<ChildSpawnSpec<M, MF>>,
  #[allow(dead_code)]
  map_system:       MapSystemShared<M>,
  actor_path:       ActorPath,
  actor_id:         ActorId,
  pid:              Pid,
  process_registry: ArcShared<ProcessRegistry<PriorityActorRef<M, MF>, ArcShared<PriorityEnvelope<M>>>>,
  watchers:         &'a mut Vec<ActorId>,
  current_priority: Option<i8>,
  receive_timeout:  Option<&'a RefCell<Box<dyn ReceiveTimeoutScheduler>>>,
  extensions:       Extensions,
  _marker:          PhantomData<M>,
}

impl<'a, M, MF, Sup> ActorContext<'a, M, MF, Sup>
where
  M: Element,
  MF: MailboxFactory + Clone,
  Sup: Supervisor<M> + ?Sized,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    mailbox_factory: &'a MF,
    mailbox_spawner: PriorityMailboxSpawnerHandle<M, MF>,
    sender: &'a MF::Producer<PriorityEnvelope<M>>,
    supervisor: &'a mut Sup,
    pending_spawns: &'a mut Vec<ChildSpawnSpec<M, MF>>,
    map_system: MapSystemShared<M>,
    actor_path: ActorPath,
    actor_id: ActorId,
    pid: Pid,
    process_registry: ArcShared<ProcessRegistry<PriorityActorRef<M, MF>, ArcShared<PriorityEnvelope<M>>>>,
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
      _marker: PhantomData,
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

  pub fn mailbox_factory(&self) -> &MF {
    self.mailbox_factory
  }

  pub(crate) fn mailbox_spawner(&self) -> &PriorityMailboxSpawnerHandle<M, MF> {
    &self.mailbox_spawner
  }

  pub(crate) fn supervisor(&mut self) -> &mut Sup {
    self.supervisor
  }

  pub fn actor_id(&self) -> ActorId {
    self.actor_id
  }

  pub fn actor_path(&self) -> &ActorPath {
    &self.actor_path
  }

  pub fn watchers(&self) -> &[ActorId] {
    self.watchers.as_slice()
  }

  pub fn pid(&self) -> &Pid {
    &self.pid
  }

  pub fn process_registry(
    &self,
  ) -> ArcShared<ProcessRegistry<PriorityActorRef<M, MF>, ArcShared<PriorityEnvelope<M>>>> {
    self.process_registry.clone()
  }

  pub fn register_watcher(&mut self, watcher: ActorId) {
    if !self.watchers.contains(&watcher) {
      self.watchers.push(watcher);
    }
  }

  pub fn unregister_watcher(&mut self, watcher: ActorId) {
    if let Some(index) = self.watchers.iter().position(|w| *w == watcher) {
      self.watchers.swap_remove(index);
    }
  }

  pub(crate) fn self_ref(&self) -> PriorityActorRef<M, MF>
  where
    MF::Queue<PriorityEnvelope<M>>: Clone,
    MF::Signal: Clone,
    MF::Producer<PriorityEnvelope<M>>: Clone, {
    PriorityActorRef::new(self.sender.clone())
  }

  #[allow(dead_code)]
  fn enqueue_spawn(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    options: MailboxOptions,
    map_system: MapSystemShared<M>,
    handler: Box<ActorHandlerFn<M, MF>>,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
  ) -> PriorityActorRef<M, MF> {
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

  pub(crate) fn spawn_child_from_props(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, MF>,
    pid_slot: ArcShared<RwLock<Option<Pid>>>,
  ) -> PriorityActorRef<M, MF>
  where
    MF: MailboxFactory + Clone + 'static, {
    let InternalProps { options, map_system, handler } = props;
    self.enqueue_spawn(supervisor, options, map_system, handler, pid_slot)
  }

  pub fn current_priority(&self) -> Option<i8> {
    self.current_priority
  }

  pub fn send_to_self_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(PriorityEnvelope::new(message, priority))
  }

  pub fn send_control_to_self(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(PriorityEnvelope::control(message, priority))
  }

  pub fn send_envelope_to_self(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.sender.try_send(envelope)
  }

  pub(crate) fn enter_priority(&mut self, priority: i8) {
    self.current_priority = Some(priority);
  }

  pub(crate) fn exit_priority(&mut self) {
    self.current_priority = None;
  }

  pub fn has_receive_timeout_scheduler(&self) -> bool {
    self.receive_timeout.is_some()
  }

  pub fn set_receive_timeout(&mut self, duration: Duration) -> bool {
    if let Some(cell) = self.receive_timeout {
      cell.borrow_mut().set(duration);
      true
    } else {
      false
    }
  }

  pub fn cancel_receive_timeout(&mut self) -> bool {
    if let Some(cell) = self.receive_timeout {
      cell.borrow_mut().cancel();
      true
    } else {
      false
    }
  }

  pub(crate) fn notify_receive_timeout_activity(&mut self, influence: bool) {
    if !influence {
      return;
    }

    if let Some(cell) = self.receive_timeout {
      cell.borrow_mut().notify_activity();
    }
  }
}
