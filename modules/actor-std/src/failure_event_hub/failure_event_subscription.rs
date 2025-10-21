use std::sync::Arc;

use super::failure_event_hub_inner::FailureEventHubInner;

/// Subscription handle to FailureEventHub. Automatically unsubscribes on Drop.
pub struct FailureEventSubscription {
  pub(super) inner: Arc<FailureEventHubInner>,
  pub(super) id:    u64,
}

impl Drop for FailureEventSubscription {
  fn drop(&mut self) {
    let mut guard = self.inner.lock_listeners();
    if let Some(index) = guard.iter().position(|(entry_id, _)| *entry_id == self.id) {
      guard.swap_remove(index);
    }
  }
}
