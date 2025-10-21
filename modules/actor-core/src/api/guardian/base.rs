use alloc::{collections::BTreeMap, format, string::String};

use cellex_utils_core_rs::QueueError;

use super::{ChildRecord, GuardianStrategy};
use crate::{
  api::{
    actor::{actor_failure::ActorFailure, actor_ref::PriorityActorRef, ActorId, ActorPath, ChildNaming, SpawnError},
    failure::FailureInfo,
    mailbox::{messages::SystemMessage, MailboxFactory, MailboxProducer},
    supervision::supervisor::SupervisorDirective,
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MapSystemShared},
  },
};

type ChildRoute<MF> = (PriorityActorRef<AnyMessage, MF>, MapSystemShared<AnyMessage>);

/// Guardian: Supervises child actors and sends SystemMessages.
pub(crate) struct Guardian<MF, Strat>
where
  MF: MailboxFactory,
  Strat: GuardianStrategy<MF>, {
  next_id:             usize,
  pub(crate) children: BTreeMap<ActorId, ChildRecord<MF>>,
  names:               BTreeMap<String, ActorId>,
  strategy:            Strat,
}

#[allow(dead_code)]
impl<MF, Strat> Guardian<MF, Strat>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  Strat: GuardianStrategy<MF>,
{
  #[allow(clippy::missing_const_for_fn)]
  pub fn new(strategy: Strat) -> Self {
    Self { next_id: 0, children: BTreeMap::new(), names: BTreeMap::new(), strategy }
  }

  #[allow(clippy::needless_pass_by_value)]
  pub fn register_child_with_naming(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
    watcher: Option<ActorId>,
    parent_path: &ActorPath,
    naming: ChildNaming,
  ) -> Result<(ActorId, ActorPath), SpawnError<AnyMessage>> {
    let assigned_name = match naming {
      | ChildNaming::Auto => None,
      | ChildNaming::WithPrefix(prefix) => Some(self.generate_prefixed_name(&prefix)),
      | ChildNaming::Explicit(name) => {
        if self.names.contains_key(&name) {
          return Err(SpawnError::name_exists(name));
        }
        Some(name)
      },
    };

    let id = ActorId(self.next_id);
    self.next_id += 1;
    self.strategy.before_start(id);
    let path = parent_path.push_child(id);
    if let Some(name) = assigned_name.as_ref() {
      self.names.insert(name.clone(), id);
    }
    self.children.insert(id, ChildRecord {
      control_ref: control_ref.clone(),
      map_system: map_system.clone(),
      watcher,
      path: path.clone(),
      name: assigned_name,
    });

    if let Some(watcher_id) = watcher {
      #[allow(clippy::redundant_clone)]
      let map_clone = map_system.clone();
      #[allow(clippy::redundant_closure)]
      let envelope = PriorityEnvelope::from_system(SystemMessage::Watch(watcher_id)).map(move |sys| map_clone(sys));
      control_ref.sender().try_send(envelope).map_err(SpawnError::from)?;
    }

    Ok((id, path))
  }

  pub fn remove_child(&mut self, id: ActorId) -> Option<PriorityActorRef<AnyMessage, MF>> {
    self.children.remove(&id).map(|record| {
      if let Some(name) = record.name.as_ref() {
        self.names.remove(name);
      }
      if let Some(watcher_id) = record.watcher {
        #[allow(clippy::redundant_clone)]
        let map_clone = record.map_system.clone();
        #[allow(clippy::redundant_closure)]
        let envelope = PriorityEnvelope::from_system(SystemMessage::Unwatch(watcher_id)).map(move |sys| map_clone(sys));
        let _ = record.control_ref.sender().try_send(envelope);
      }
      record.control_ref
    })
  }

  pub fn child_ref(&self, id: ActorId) -> Option<&PriorityActorRef<AnyMessage, MF>> {
    self.children.get(&id).map(|record| &record.control_ref)
  }

  pub fn notify_failure(
    &mut self,
    actor: ActorId,
    failure: ActorFailure,
  ) -> Result<Option<FailureInfo>, QueueError<PriorityEnvelope<AnyMessage>>> {
    let path = match self.children.get(&actor) {
      | Some(record) => record.path.clone(),
      | None => ActorPath::new().push_child(actor),
    };
    let directive = self.strategy.decide(actor, failure.behavior());
    let failure = FailureInfo::from_failure(actor, path, failure);
    self.handle_directive(actor, failure, directive)
  }

  pub fn stop_child(&mut self, actor: ActorId) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    if let Some(record) = self.children.get(&actor) {
      let map_clone = record.map_system.clone();
      #[allow(clippy::redundant_closure)]
      let envelope = PriorityEnvelope::from_system(SystemMessage::Stop).map(move |sys| map_clone(sys));
      record.control_ref.sender().try_send(envelope)
    } else {
      Ok(())
    }
  }

  pub fn escalate_failure(
    &mut self,
    failure: FailureInfo,
  ) -> Result<Option<FailureInfo>, QueueError<PriorityEnvelope<AnyMessage>>> {
    let actor = failure.actor;
    let directive = self.strategy.decide(actor, failure.behavior_failure());
    self.handle_directive(actor, failure, directive)
  }

  pub fn register_child(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
    watcher: Option<ActorId>,
    parent_path: &ActorPath,
  ) -> Result<(ActorId, ActorPath), QueueError<PriorityEnvelope<AnyMessage>>> {
    match self.register_child_with_naming(control_ref, map_system, watcher, parent_path, ChildNaming::Auto) {
      | Ok(result) => Ok(result),
      | Err(SpawnError::Queue(err)) => Err(err),
      | Err(SpawnError::NameExists(name)) => {
        debug_assert!(false, "auto-generated actor name unexpectedly conflicted: {name}");
        Err(QueueError::Disconnected)
      },
    }
  }

  pub fn child_route(&self, actor: ActorId) -> Option<ChildRoute<MF>> {
    self.children.get(&actor).map(|record| (record.control_ref.clone(), record.map_system.clone()))
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
  ) -> Result<Option<FailureInfo>, QueueError<PriorityEnvelope<AnyMessage>>> {
    match directive {
      | SupervisorDirective::Resume => Ok(None),
      | SupervisorDirective::Stop => {
        if let Some(record) = self.children.get(&actor) {
          #[allow(clippy::redundant_clone)]
          let map_clone = record.map_system.clone();
          #[allow(clippy::redundant_closure)]
          let envelope = PriorityEnvelope::from_system(SystemMessage::Stop).map(move |sys| map_clone(sys));
          record.control_ref.sender().try_send(envelope)?;
          Ok(None)
        } else {
          Ok(Some(failure))
        }
      },
      | SupervisorDirective::Restart => {
        if let Some(record) = self.children.get(&actor) {
          #[allow(clippy::redundant_clone)]
          let map_clone = record.map_system.clone();
          #[allow(clippy::redundant_closure)]
          let envelope = PriorityEnvelope::from_system(SystemMessage::Restart).map(move |sys| map_clone(sys));
          record.control_ref.sender().try_send(envelope)?;
          self.strategy.after_restart(actor);
          Ok(None)
        } else {
          Ok(Some(failure))
        }
      },
      | SupervisorDirective::Escalate => {
        if let Some(parent_failure) = failure.escalate_to_parent() {
          Ok(Some(parent_failure))
        } else {
          Ok(Some(failure))
        }
      },
    }
  }
}
