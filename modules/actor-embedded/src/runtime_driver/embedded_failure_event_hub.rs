use alloc::vec::Vec;

use cellex_actor_core_rs::api::{
  failure_event_stream::FailureEventStream,
  supervision::{escalation::FailureEventListener, failure::FailureEvent},
};
use cellex_utils_core_rs::sync::ArcShared;
use spin::Mutex;

use super::{shared::EmbeddedFailureEventHubState, EmbeddedFailureEventSubscription};

/// Simple FailureEventHub implementation for embedded environments.
#[derive(Clone)]
pub struct EmbeddedFailureEventHub {
  pub(super) inner: ArcShared<Mutex<EmbeddedFailureEventHubState>>,
}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Send for EmbeddedFailureEventHub {}

#[cfg(not(target_has_atomic = "ptr"))]
unsafe impl Sync for EmbeddedFailureEventHub {}

impl EmbeddedFailureEventHub {
  /// Creates a new `EmbeddedFailureEventHub`.
  ///
  /// # Returns
  ///
  /// A new event hub instance
  #[must_use]
  pub fn new() -> Self {
    Self { inner: ArcShared::new(Mutex::new(EmbeddedFailureEventHubState::default())) }
  }

  pub(super) fn snapshot_listeners(&self) -> Vec<FailureEventListener> {
    let locked = self.inner.lock();
    locked.listeners.iter().map(|(_, listener)| listener.clone()).collect()
  }
}

impl Default for EmbeddedFailureEventHub {
  fn default() -> Self {
    Self::new()
  }
}

impl FailureEventStream for EmbeddedFailureEventHub {
  type Subscription = EmbeddedFailureEventSubscription;

  fn listener(&self) -> FailureEventListener {
    let inner = self.clone();
    FailureEventListener::new(move |event: FailureEvent| {
      for listener in inner.snapshot_listeners().into_iter() {
        listener(event.clone());
      }
    })
  }

  fn subscribe(&self, listener: FailureEventListener) -> Self::Subscription {
    let id = {
      let mut state = self.inner.lock();
      let id = state.next_id;
      state.next_id = state.next_id.wrapping_add(1);
      state.listeners.push((id, listener));
      id
    };

    EmbeddedFailureEventSubscription { inner: self.inner.clone(), id }
  }
}
