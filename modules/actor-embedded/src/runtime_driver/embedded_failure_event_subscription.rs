use cellex_utils_core_rs::sync::ArcShared;
use spin::Mutex;

use super::shared::EmbeddedFailureEventHubState;

/// Subscription handle for failure events in embedded environments.
///
/// Automatically unsubscribes when dropped.
pub struct EmbeddedFailureEventSubscription {
  pub(super) inner: ArcShared<Mutex<EmbeddedFailureEventHubState>>,
  pub(super) id:    u64,
}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Send for EmbeddedFailureEventSubscription {}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Sync for EmbeddedFailureEventSubscription {}

impl Drop for EmbeddedFailureEventSubscription {
  fn drop(&mut self) {
    let mut state = self.inner.lock();
    if let Some(pos) = state.listeners.iter().position(|(entry_id, _)| *entry_id == self.id) {
      state.listeners.swap_remove(pos);
    }
  }
}
