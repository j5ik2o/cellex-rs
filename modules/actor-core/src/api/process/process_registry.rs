use alloc::{
  collections::BTreeMap,
  string::{String, ToString},
};
use core::fmt;

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use super::{
  dead_letter::{DeadLetter, DeadLetterHub, DeadLetterReason},
  pid::{NodeId, Pid, SystemId},
};
use crate::api::actor::ActorPath;

#[cfg(test)]
mod tests;

/// Result of resolving a PID within the registry.
#[derive(Clone)]
pub enum ProcessResolution<T> {
  /// The PID maps to a local process handle.
  Local(ArcShared<T>),
  /// The PID belongs to a remote node.
  Remote,
  /// No process is registered for the PID.
  Unresolved,
}

impl<T> fmt::Debug for ProcessResolution<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | Self::Local(_) => f.write_str("Local(..)"),
      | Self::Remote => f.write_str("Remote"),
      | Self::Unresolved => f.write_str("Unresolved"),
    }
  }
}

/// Registry maintaining PID → process handle mappings and a DeadLetter hub.
pub struct ProcessRegistry<T, M> {
  system:       SystemId,
  node:         Option<NodeId>,
  processes:    RwLock<BTreeMap<String, ArcShared<T>>>,
  dead_letters: RwLock<DeadLetterHub<M>>,
}

impl<T, M> ProcessRegistry<T, M> {
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
  pub fn register_local(&self, path: ActorPath, handle: ArcShared<T>) -> Pid {
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
  pub fn resolve_pid(&self, pid: &Pid) -> ProcessResolution<T> {
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
  pub fn resolve_or_dead_letter(&self, pid: &Pid, message: M, reason: DeadLetterReason) -> Option<ArcShared<T>> {
    self.resolve_or_dead_letter_with_remote(pid, message, reason, DeadLetterReason::NetworkUnreachable)
  }

  /// Resolves the PID and publishes dead letters for unresolved or remote targets.
  pub fn resolve_or_dead_letter_with_remote(
    &self,
    pid: &Pid,
    message: M,
    unresolved_reason: DeadLetterReason,
    remote_reason: DeadLetterReason,
  ) -> Option<ArcShared<T>> {
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
  pub fn subscribe_dead_letters(&self, listener: ArcShared<super::dead_letter::DeadLetterListener<M>>) {
    self.dead_letters.write().subscribe(listener);
  }

  /// Publishes a dead letter directly.
  pub fn publish_dead_letter(&self, dead_letter: &DeadLetter<M>) {
    let hub = self.dead_letters.read();
    hub.publish(dead_letter);
  }
}

impl<T, M> Default for ProcessRegistry<T, M> {
  fn default() -> Self {
    Self::new(SystemId::new("cellex"), None)
  }
}
