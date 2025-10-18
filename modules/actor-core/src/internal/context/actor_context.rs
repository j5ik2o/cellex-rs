use alloc::{boxed::Box, vec, vec::Vec};
use core::{cell::RefCell, marker::PhantomData, time::Duration};

use cellex_utils_core_rs::{Element, QueueError, QueueSize};

use super::ChildSpawnSpec;
use crate::{
  api::{
    actor::{actor_failure::ActorFailure, actor_ref::PriorityActorRef, ActorId, ActorPath, ChildNaming},
    actor_system::map_system::MapSystemShared,
    extensions::{Extension, ExtensionId, Extensions},
    mailbox::{MailboxFactory, MailboxOptions, MailboxProducer, PriorityEnvelope},
    receive_timeout::ReceiveTimeoutScheduler,
    supervision::supervisor::Supervisor,
  },
  internal::{actor::InternalProps, mailbox::PriorityMailboxSpawnerHandle},
};

/// Type alias representing the dynamically-dispatched actor handler invoked by schedulers.
pub type ActorHandlerFn<M, MF> =
  dyn for<'ctx> FnMut(&mut ActorContext<'ctx, M, MF, dyn Supervisor<M>>, M) -> Result<(), ActorFailure> + 'static;
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

  pub fn mailbox_spawner(&self) -> &PriorityMailboxSpawnerHandle<M, MF> {
    &self.mailbox_spawner
  }

  pub fn supervisor(&mut self) -> &mut Sup {
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
    });
    actor_ref
  }

  pub(crate) fn spawn_child<F, S>(
    &mut self,
    supervisor: S,
    options: MailboxOptions,
    handler: F,
  ) -> PriorityActorRef<M, MF>
  where
    F: for<'ctx> FnMut(&mut ActorContext<'ctx, M, MF, dyn Supervisor<M>>, M) + 'static,
    S: Supervisor<M> + 'static, {
    let mut handler = handler;
    self.enqueue_spawn(
      Box::new(supervisor),
      options,
      self.map_system.clone(),
      Box::new(move |ctx, message| {
        handler(ctx, message);
        Ok(())
      }),
    )
  }

  pub(crate) fn spawn_child_from_props(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    props: InternalProps<M, MF>,
  ) -> PriorityActorRef<M, MF>
  where
    MF: MailboxFactory + Clone + 'static, {
    let InternalProps { options, map_system, handler } = props;
    self.enqueue_spawn(supervisor, options, map_system, handler)
  }

  #[allow(dead_code)]
  pub(crate) fn spawn_control_child<F, S>(&mut self, supervisor: S, handler: F) -> PriorityActorRef<M, MF>
  where
    F: for<'ctx> FnMut(&mut ActorContext<'ctx, M, MF, dyn Supervisor<M>>, M) + 'static,
    S: Supervisor<M> + 'static, {
    let options = MailboxOptions::default().with_priority_capacity(QueueSize::limitless());
    self.spawn_child(supervisor, options, handler)
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
