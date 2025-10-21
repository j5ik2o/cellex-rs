//! Dead letter hub implementation.

use alloc::vec::Vec;

use cellex_utils_core_rs::sync::ArcShared;

use crate::api::process::dead_letter::DeadLetter;

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
