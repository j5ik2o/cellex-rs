use std::sync::{atomic::AtomicU64, Mutex, MutexGuard};

use cellex_actor_core_rs::api::supervision::escalation::FailureEventListener;

pub(super) struct FailureEventHubInner {
  pub(super) next_id:   AtomicU64,
  pub(super) listeners: Mutex<Vec<(u64, FailureEventListener)>>,
}

impl Default for FailureEventHubInner {
  fn default() -> Self {
    Self { next_id: AtomicU64::new(1), listeners: Mutex::new(Vec::new()) }
  }
}

impl FailureEventHubInner {
  pub(super) fn lock_listeners(&self) -> MutexGuard<'_, Vec<(u64, FailureEventListener)>> {
    self.listeners.lock().unwrap_or_else(|err| err.into_inner())
  }
}
