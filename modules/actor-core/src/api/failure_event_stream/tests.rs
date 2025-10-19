#![allow(clippy::disallowed_types)]
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::FailureEventStream;

use crate::api::supervision::{escalation::FailureEventListener, failure::FailureEvent};

/// In-memory implementation for testing only.
#[derive(Clone, Default)]
pub(crate) struct TestFailureEventStream {
  inner: Arc<TestFailureEventStreamInner>,
}

#[derive(Default)]
struct TestFailureEventStreamInner {
  next_id:   AtomicU64,
  listeners: Mutex<Vec<(u64, FailureEventListener)>>,
}

#[derive(Clone)]
pub(crate) struct TestFailureEventSubscription {
  inner: Arc<TestFailureEventStreamInner>,
  id:    u64,
}

impl FailureEventStream for TestFailureEventStream {
  type Subscription = TestFailureEventSubscription;

  fn listener(&self) -> FailureEventListener {
    let inner = self.inner.clone();
    FailureEventListener::new(move |event: FailureEvent| {
      let snapshot: Vec<FailureEventListener> = {
        let guard = inner.listeners.lock().unwrap();
        guard.iter().map(|(_, listener)| listener.clone()).collect()
      };
      for listener in snapshot.into_iter() {
        listener(event.clone());
      }
    })
  }

  fn subscribe(&self, listener: FailureEventListener) -> Self::Subscription {
    let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
    {
      let mut guard = self.inner.listeners.lock().unwrap();
      guard.push((id, listener));
    }
    TestFailureEventSubscription { inner: self.inner.clone(), id }
  }
}

impl Drop for TestFailureEventSubscription {
  fn drop(&mut self) {
    if let Ok(mut guard) = self.inner.listeners.lock() {
      if let Some(index) = guard.iter().position(|(slot_id, _)| *slot_id == self.id) {
        guard.swap_remove(index);
      }
    }
  }
}
