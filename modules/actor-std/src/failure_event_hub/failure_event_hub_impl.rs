use std::sync::{atomic::Ordering, Arc};

use cellex_actor_core_rs::api::failure::failure_event_stream::{FailureEventListener, FailureEventStream};
use cellex_actor_core_rs::api::failure::FailureEvent;
use super::{failure_event_hub_inner::FailureEventHubInner, FailureEventSubscription};

/// FailureEventStream implementation for std.
#[derive(Clone, Default)]
pub struct FailureEventHub {
  pub(super) inner: Arc<FailureEventHubInner>,
}

impl FailureEventHub {
  /// Creates a new `FailureEventHub` instance.
  ///
  /// # Returns
  ///
  /// A new hub instance initialized with default state
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  fn notify_listeners(&self, event: &FailureEvent) {
    let snapshot: Vec<FailureEventListener> = {
      let guard = self.inner.lock_listeners();
      guard.iter().map(|(_, listener)| listener.clone()).collect()
    };
    for listener in snapshot.into_iter() {
      listener(event.clone());
    }
  }
}

impl FailureEventStream for FailureEventHub {
  type Subscription = FailureEventSubscription;

  fn listener(&self) -> FailureEventListener {
    let inner = self.clone();
    FailureEventListener::new(move |event: FailureEvent| inner.notify_listeners(&event))
  }

  fn subscribe(&self, listener: FailureEventListener) -> Self::Subscription {
    let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
    {
      let mut guard = self.inner.lock_listeners();
      guard.push((id, listener));
    }

    FailureEventSubscription { inner: self.inner.clone(), id }
  }
}
