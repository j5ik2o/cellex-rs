//! Tokio async barrier backend implementation.

#![allow(clippy::disallowed_types)]
use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

use async_trait::async_trait;
use cellex_utils_core_rs::concurrent::async_barrier::AsyncBarrierBackend;
use tokio::sync::Notify;

struct Inner {
  remaining: AtomicUsize,
  initial:   usize,
  notify:    Notify,
}

/// Backend implementation of async barrier using Tokio runtime
///
/// A synchronization primitive for multiple tasks to wait for each other.
/// Blocks all tasks until the specified number of tasks call `wait()`.
#[derive(Clone)]
pub struct TokioAsyncBarrierBackend {
  inner: Arc<Inner>,
}

#[async_trait(?Send)]
impl AsyncBarrierBackend for TokioAsyncBarrierBackend {
  fn new(count: usize) -> Self {
    assert!(count > 0, "AsyncBarrier must have positive count");
    Self { inner: Arc::new(Inner { remaining: AtomicUsize::new(count), initial: count, notify: Notify::new() }) }
  }

  async fn wait(&self) {
    let inner = self.inner.clone();
    let prev = inner.remaining.fetch_sub(1, Ordering::SeqCst);
    assert!(prev > 0, "AsyncBarrier::wait called more times than count");
    if prev == 1 {
      inner.remaining.store(inner.initial, Ordering::SeqCst);
      inner.notify.notify_waiters();
    } else {
      loop {
        if inner.remaining.load(Ordering::SeqCst) == inner.initial {
          break;
        }
        inner.notify.notified().await;
      }
    }
  }
}
