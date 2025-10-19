#[cfg(test)]
mod tests;

use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

use async_trait::async_trait;
use cellex_utils_core_rs::{WaitGroup as CoreWaitGroup, WaitGroupBackend};
use tokio::sync::Notify;

/// Backend implementation of WaitGroup using Tokio runtime
///
/// Used for synchronizing async tasks, allowing waiting until multiple async operations complete.
#[derive(Clone)]
pub struct TokioWaitGroupBackend {
  inner: Arc<Inner>,
}

struct Inner {
  count:  AtomicUsize,
  notify: Notify,
}

#[async_trait(?Send)]
impl WaitGroupBackend for TokioWaitGroupBackend {
  fn new() -> Self {
    Self::with_count(0)
  }

  fn with_count(count: usize) -> Self {
    Self { inner: Arc::new(Inner { count: AtomicUsize::new(count), notify: Notify::new() }) }
  }

  fn add(&self, n: usize) {
    self.inner.count.fetch_add(n, Ordering::SeqCst);
  }

  fn done(&self) {
    let prev = self.inner.count.fetch_sub(1, Ordering::SeqCst);
    assert!(prev > 0, "WaitGroup::done called more times than add");
    if prev == 1 {
      self.inner.notify.notify_waiters();
    }
  }

  async fn wait(&self) {
    let inner = self.inner.clone();
    loop {
      if inner.count.load(Ordering::SeqCst) == 0 {
        return;
      }
      inner.notify.notified().await;
    }
  }
}

/// Type alias for WaitGroup using Tokio backend
///
/// A synchronization primitive for waiting until multiple async tasks complete.
pub type WaitGroup = CoreWaitGroup<TokioWaitGroupBackend>;
