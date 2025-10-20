use alloc::vec::Vec;
use core::fmt;

use cellex_utils_core_rs::sync::ArcShared;

use super::pid::Pid;

#[cfg(test)]
mod tests;

/// Reason why a message was routed to dead letters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadLetterReason {
  /// No process is registered for the given PID.
  UnregisteredPid,
  /// Process exists but is currently terminating or terminated.
  Terminated,
  /// The delivery subsystem rejected the message (e.g., queue full).
  DeliveryRejected,
  /// Remote transport reported a network-level failure for the destination node.
  NetworkUnreachable,
  /// Custom reason text supplied by the caller.
  Custom(&'static str),
}

impl fmt::Display for DeadLetterReason {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | Self::UnregisteredPid => f.write_str("unregistered pid"),
      | Self::Terminated => f.write_str("terminated"),
      | Self::DeliveryRejected => f.write_str("delivery rejected"),
      | Self::NetworkUnreachable => f.write_str("network unreachable"),
      | Self::Custom(msg) => f.write_str(msg),
    }
  }
}

/// Message captured by the DeadLetter hub.
#[derive(Debug, Clone)]
pub struct DeadLetter<M> {
  /// PID originally targeted by the message.
  pub pid:     Pid,
  /// Original message envelope.
  pub message: M,
  /// Recorded reason.
  pub reason:  DeadLetterReason,
}

impl<M> DeadLetter<M> {
  /// Creates a new dead letter entry.
  pub const fn new(pid: Pid, message: M, reason: DeadLetterReason) -> Self {
    Self { pid, message, reason }
  }
}

/// Listener invoked when a dead letter is published.
#[cfg(target_has_atomic = "ptr")]
pub type DeadLetterListener<M> = dyn Fn(&DeadLetter<M>) + Send + Sync + 'static;

/// Listener invoked when a dead letter is published.
#[cfg(not(target_has_atomic = "ptr"))]
pub type DeadLetterListener<M> = dyn Fn(&DeadLetter<M>) + 'static;

/// Hub that dispatches dead letters to interested observers.
pub struct DeadLetterHub<M> {
  listeners: Vec<ArcShared<DeadLetterListener<M>>>,
}

impl<M> DeadLetterHub<M> {
  /// Creates an empty hub.
  #[must_use]
  pub const fn new() -> Self {
    Self { listeners: Vec::new() }
  }

  /// Subscribes a listener to future dead letters.
  pub fn subscribe(&mut self, listener: ArcShared<DeadLetterListener<M>>) {
    self.listeners.push(listener);
  }

  /// Publishes a dead letter to all listeners.
  pub fn publish(&self, dead_letter: &DeadLetter<M>) {
    for listener in &self.listeners {
      listener(dead_letter);
    }
  }

  /// Returns true if there are listeners registered.
  #[must_use]
  pub const fn has_listeners(&self) -> bool {
    !self.listeners.is_empty()
  }
}

impl<M> Default for DeadLetterHub<M> {
  fn default() -> Self {
    Self::new()
  }
}
