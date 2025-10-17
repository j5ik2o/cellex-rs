use super::{ChildRecord, GuardianStrategy};
use crate::api::actor::failure::ActorFailure;
use crate::api::identity::ActorId;
use crate::api::identity::ActorPath;
use crate::api::mailbox::MailboxProducer;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::mailbox::SystemMessage;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::supervisor::SupervisorDirective;
use crate::internal::actor::InternalActorRef;
use crate::internal::scheduler::ChildNaming;
use crate::internal::scheduler::SpawnError;
use crate::shared::map_system::MapSystemShared;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use cellex_utils_core_rs::{Element, QueueError};

type ChildRoute<M, R> = (InternalActorRef<M, R>, MapSystemShared<M>);

/// Guardian: Supervises child actors and sends SystemMessages.
pub(crate) struct Guardian<M, R, Strat>
where
  M: Element,
  R: MailboxFactory,
  Strat: GuardianStrategy<M, R>, {
  next_id: usize,
  pub(crate) children: BTreeMap<ActorId, ChildRecord<M, R>>,
  names: BTreeMap<String, ActorId>,
  strategy: Strat,
  _marker: core::marker::PhantomData<M>,
}

#[allow(dead_code)]
impl<M, R, Strat> Guardian<M, R, Strat>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  pub fn new(strategy: Strat) -> Self {
    Self {
      next_id: 0,
      children: BTreeMap::new(),
      names: BTreeMap::new(),
      strategy,
      _marker: core::marker::PhantomData,
    }
  }

  pub fn register_child_with_naming(
    &mut self,
    control_ref: InternalActorRef<M, R>,
    map_system: MapSystemShared<M>,
    watcher: Option<ActorId>,
    parent_path: &ActorPath,
    naming: ChildNaming,
  ) -> Result<(ActorId, ActorPath), SpawnError<M>> {
    let assigned_name = match naming {
      ChildNaming::Auto => None,
      ChildNaming::WithPrefix(prefix) => Some(self.generate_prefixed_name(&prefix)),
      ChildNaming::Explicit(name) => {
        if self.names.contains_key(&name) {
          return Err(SpawnError::name_exists(name));
        }
        Some(name)
      }
    };

    let id = ActorId(self.next_id);
    self.next_id += 1;
    self.strategy.before_start(id);
    let path = parent_path.push_child(id);
    if let Some(name) = assigned_name.as_ref() {
      self.names.insert(name.clone(), id);
    }
    self.children.insert(
      id,
      ChildRecord {
        control_ref: control_ref.clone(),
        map_system: map_system.clone(),
        watcher,
        path: path.clone(),
        name: assigned_name,
      },
    );

    if let Some(watcher_id) = watcher {
      let map_clone = map_system.clone();
      let envelope = PriorityEnvelope::from_system(SystemMessage::Watch(watcher_id)).map(move |sys| (map_clone)(sys));
      control_ref.sender().try_send(envelope).map_err(SpawnError::from)?;
    }

    Ok((id, path))
  }

  pub fn remove_child(&mut self, id: ActorId) -> Option<InternalActorRef<M, R>> {
    self.children.remove(&id).map(|record| {
      if let Some(name) = record.name.as_ref() {
        self.names.remove(name);
      }
      if let Some(watcher_id) = record.watcher {
        let map_clone = record.map_system.clone();
        let envelope =
          PriorityEnvelope::from_system(SystemMessage::Unwatch(watcher_id)).map(move |sys| (map_clone)(sys));
        let _ = record.control_ref.sender().try_send(envelope);
      }
      record.control_ref
    })
  }

  pub fn child_ref(&self, id: ActorId) -> Option<&InternalActorRef<M, R>> {
    self.children.get(&id).map(|record| &record.control_ref)
  }

  pub fn notify_failure(
    &mut self,
    actor: ActorId,
    failure: ActorFailure,
  ) -> Result<Option<FailureInfo>, QueueError<PriorityEnvelope<M>>> {
    let path = match self.children.get(&actor) {
      Some(record) => record.path.clone(),
      None => ActorPath::new().push_child(actor),
    };
    let directive = self.strategy.decide(actor, failure.behavior());
    let failure = FailureInfo::from_failure(actor, path, failure);
    self.handle_directive(actor, failure, directive)
  }

  pub fn stop_child(&mut self, actor: ActorId) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    if let Some(record) = self.children.get(&actor) {
      let envelope = PriorityEnvelope::from_system(SystemMessage::Stop).map(|sys| (&*record.map_system)(sys));
      record.control_ref.sender().try_send(envelope)
    } else {
      Ok(())
    }
  }

  pub fn escalate_failure(
    &mut self,
    failure: FailureInfo,
  ) -> Result<Option<FailureInfo>, QueueError<PriorityEnvelope<M>>> {
    let actor = failure.actor;
    let directive = self.strategy.decide(actor, failure.behavior_failure());
    self.handle_directive(actor, failure, directive)
  }

  pub fn register_child(
    &mut self,
    control_ref: InternalActorRef<M, R>,
    map_system: MapSystemShared<M>,
    watcher: Option<ActorId>,
    parent_path: &ActorPath,
  ) -> Result<(ActorId, ActorPath), QueueError<PriorityEnvelope<M>>> {
    match self.register_child_with_naming(control_ref, map_system, watcher, parent_path, ChildNaming::Auto) {
      Ok(result) => Ok(result),
      Err(SpawnError::Queue(err)) => Err(err),
      Err(SpawnError::NameExists(_)) => {
        unreachable!("NameExists cannot occur when using automatic naming")
      }
    }
  }

  pub fn child_route(&self, actor: ActorId) -> Option<ChildRoute<M, R>> {
    self
      .children
      .get(&actor)
      .map(|record| (record.control_ref.clone(), record.map_system.clone()))
  }

  fn generate_prefixed_name(&mut self, prefix: &str) -> String {
    let mut attempt = 0usize;
    loop {
      let candidate = format!("{prefix}-{}", self.next_id + attempt);
      if !self.names.contains_key(&candidate) {
        return candidate;
      }
      attempt = attempt.saturating_add(1);
    }
  }

  fn handle_directive(
    &mut self,
    actor: ActorId,
    failure: FailureInfo,
    directive: SupervisorDirective,
  ) -> Result<Option<FailureInfo>, QueueError<PriorityEnvelope<M>>> {
    match directive {
      SupervisorDirective::Resume => Ok(None),
      SupervisorDirective::Stop => {
        if let Some(record) = self.children.get(&actor) {
          let envelope = PriorityEnvelope::from_system(SystemMessage::Stop).map(|sys| (&*record.map_system)(sys));
          record.control_ref.sender().try_send(envelope)?;
          Ok(None)
        } else {
          Ok(Some(failure))
        }
      }
      SupervisorDirective::Restart => {
        if let Some(record) = self.children.get(&actor) {
          let envelope = PriorityEnvelope::from_system(SystemMessage::Restart).map(|sys| (&*record.map_system)(sys));
          record.control_ref.sender().try_send(envelope)?;
          self.strategy.after_restart(actor);
          Ok(None)
        } else {
          Ok(Some(failure))
        }
      }
      SupervisorDirective::Escalate => {
        if let Some(parent_failure) = failure.escalate_to_parent() {
          Ok(Some(parent_failure))
        } else {
          Ok(Some(failure))
        }
      }
    }
  }
}
