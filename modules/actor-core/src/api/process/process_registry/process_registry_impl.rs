//! Process registry implementation.

use alloc::{
  collections::BTreeMap,
  string::{String, ToString},
};

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use crate::api::{
  actor::ActorPath,
  process::{
    dead_letter::{DeadLetter, DeadLetterHub, DeadLetterListener, DeadLetterReason},
    pid::{NodeId, Pid, SystemId},
    process_registry::ProcessResolution,
  },
};

/// Registry maintaining PID → process handle mappings and a DeadLetter hub.
pub struct ProcessRegistry<P, M> {
  system:       SystemId,
  node:         Option<NodeId>,
  processes:    RwLock<BTreeMap<String, ArcShared<P>>>,
  dead_letters: RwLock<DeadLetterHub<M>>,
}

impl<P, M> ProcessRegistry<P, M> {
  /// Creates a new process registry for the given system/node combination.
  #[must_use]
  pub const fn new(system: SystemId, node: Option<NodeId>) -> Self {
    Self { system, node, processes: RwLock::new(BTreeMap::new()), dead_letters: RwLock::new(DeadLetterHub::new()) }
  }

  fn pid_key(pid: &Pid) -> String {
    pid.to_string()
  }

  /// Returns the system identifier.
  #[must_use]
  pub const fn system(&self) -> &SystemId {
    &self.system
  }

  /// Returns the local node identifier.
  #[must_use]
  pub const fn node(&self) -> Option<&NodeId> {
    self.node.as_ref()
  }

  /// Registers a local process handle and returns its PID.
  pub fn register_local(&self, path: ActorPath, handle: ArcShared<P>) -> Pid {
    let pid = match self.node.clone() {
      | Some(node) => Pid::new(self.system.clone(), path).with_node(node),
      | None => Pid::new(self.system.clone(), path),
    };

    self.processes.write().insert(Self::pid_key(&pid), handle);
    pid
  }

  /// Removes a process entry.
  pub fn deregister(&self, pid: &Pid) {
    self.processes.write().remove(&Self::pid_key(pid));
  }

  /// Resolves a PID to a process handle, remote indicator, or unresolved.
  pub fn resolve_pid(&self, pid: &Pid) -> ProcessResolution<P> {
    if pid.system() != &self.system {
      return ProcessResolution::Remote;
    }

    if pid.node() != self.node.as_ref() {
      return ProcessResolution::Remote;
    }

    match self.processes.read().get(&Self::pid_key(pid)) {
      | Some(handle) => ProcessResolution::Local(handle.clone()),
      | None => ProcessResolution::Unresolved,
    }
  }

  /// Resolves the PID and, if not found, records a dead letter entry.
  pub fn resolve_or_dead_letter(&self, pid: &Pid, message: M, reason: DeadLetterReason) -> Option<ArcShared<P>> {
    self.resolve_or_dead_letter_with_remote(pid, message, reason, DeadLetterReason::NetworkUnreachable)
  }

  /// Resolves the PID and publishes dead letters for unresolved or remote targets.
  pub fn resolve_or_dead_letter_with_remote(
    &self,
    pid: &Pid,
    message: M,
    unresolved_reason: DeadLetterReason,
    remote_reason: DeadLetterReason,
  ) -> Option<ArcShared<P>> {
    match self.resolve_pid(pid) {
      | ProcessResolution::Local(handle) => {
        let _ = message;
        Some(handle)
      },
      | ProcessResolution::Remote => {
        let letter = DeadLetter::new(pid.clone(), message, remote_reason);
        self.publish_dead_letter(&letter);
        None
      },
      | ProcessResolution::Unresolved => {
        let letter = DeadLetter::new(pid.clone(), message, unresolved_reason);
        self.publish_dead_letter(&letter);
        None
      },
    }
  }

  /// Subscribes a listener to the dead letter hub.
  pub fn subscribe_dead_letters(&self, listener: ArcShared<DeadLetterListener<M>>) {
    self.dead_letters.write().subscribe(listener);
  }

  /// Publishes a dead letter directly.
  pub fn publish_dead_letter(&self, dead_letter: &DeadLetter<M>) {
    let hub = self.dead_letters.read();
    hub.publish(dead_letter);
  }
}

impl<P, M> Default for ProcessRegistry<P, M> {
  fn default() -> Self {
    Self::new(SystemId::new("cellex"), None)
  }
}
