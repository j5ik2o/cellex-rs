#[cfg(test)]
mod tests;

use std::sync::{
  atomic::{AtomicU64, Ordering},
  Arc, Mutex,
};

use cellex_actor_core_rs::api::{
  failure_event_stream::FailureEventStream,
  supervision::{escalation::FailureEventListener, failure::FailureEvent},
};

/// FailureEventStream implementation for std.
#[derive(Clone, Default)]
pub struct FailureEventHub {
  inner: Arc<FailureEventHubInner>,
}

struct FailureEventHubInner {
  next_id:   AtomicU64,
  listeners: Mutex<Vec<(u64, FailureEventListener)>>,
}

impl Default for FailureEventHubInner {
  fn default() -> Self {
    Self { next_id: AtomicU64::new(1), listeners: Mutex::new(Vec::new()) }
  }
}

impl FailureEventHub {
  /// Creates a new `FailureEventHub` instance.
  ///
  /// # Returns
  ///
  /// A new hub instance initialized with default state
  pub fn new() -> Self {
    Self::default()
  }

  fn notify_listeners(&self, event: FailureEvent) {
    let snapshot: Vec<FailureEventListener> = {
      let guard = self.inner.listeners.lock().unwrap();
      guard.iter().map(|(_, listener)| listener.clone()).collect()
    };
    for listener in snapshot.into_iter() {
      listener(event.clone());
    }
  }

  #[cfg(test)]
  pub(crate) fn listener_count(&self) -> usize {
    let guard = self.inner.listeners.lock().unwrap();
    guard.len()
  }
}

impl FailureEventStream for FailureEventHub {
  type Subscription = FailureEventSubscription;

  fn listener(&self) -> FailureEventListener {
    let inner = self.clone();
    FailureEventListener::new(move |event: FailureEvent| inner.notify_listeners(event))
  }

  fn subscribe(&self, listener: FailureEventListener) -> Self::Subscription {
    let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
    {
      let mut guard = self.inner.listeners.lock().unwrap();
      guard.push((id, listener));
    }

    FailureEventSubscription { inner: self.inner.clone(), id }
  }
}

/// Subscription handle to FailureEventHub. Automatically unsubscribes on Drop.
pub struct FailureEventSubscription {
  inner: Arc<FailureEventHubInner>,
  id:    u64,
}

impl Drop for FailureEventSubscription {
  fn drop(&mut self) {
    let mut guard = self.inner.listeners.lock().unwrap();
    if let Some(index) = guard.iter().position(|(entry_id, _)| *entry_id == self.id) {
      guard.swap_remove(index);
    }
  }
}
