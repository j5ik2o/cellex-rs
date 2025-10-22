#![allow(clippy::disallowed_types)]
#![cfg(feature = "arc")]


use alloc::{boxed::Box, sync::Arc};
use core::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use cellex_utils_core_rs::{WaitGroup as CoreWaitGroup, WaitGroupBackend};
use embassy_sync::{
  blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
  signal::Signal,
};

/// Backend implementation for wait group using `Arc`
///
/// Provides wait group synchronization using atomic operations and embassy-sync
/// signals with `Arc` for thread-safe reference counting. Threads wait until all
/// tasks complete (count reaches zero).
///
/// # Type Parameters
///
/// * `RM` - Raw mutex type from embassy-sync
pub struct ArcWaitGroupBackend<RM>
where
  RM: RawMutex, {
  count:  Arc<AtomicUsize>,
  signal: Arc<Signal<RM, ()>>,
}

impl<RM> Clone for ArcWaitGroupBackend<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self { count: self.count.clone(), signal: self.signal.clone() }
  }
}

#[async_trait(?Send)]
impl<RM> WaitGroupBackend for ArcWaitGroupBackend<RM>
where
  RM: RawMutex + Send + Sync,
{
  fn new() -> Self {
    Self::with_count(0)
  }

  fn with_count(count: usize) -> Self {
    Self { count: Arc::new(AtomicUsize::new(count)), signal: Arc::new(Signal::new()) }
  }

  fn add(&self, n: usize) {
    self.count.fetch_add(n, Ordering::SeqCst);
  }

  fn done(&self) {
    let prev = self.count.fetch_sub(1, Ordering::SeqCst);
    assert!(prev > 0, "WaitGroup::done called more times than add");
    if prev == 1 {
      self.signal.signal(());
    }
  }

  async fn wait(&self) {
    let count = self.count.clone();
    let signal = self.signal.clone();
    loop {
      if count.load(Ordering::SeqCst) == 0 {
        return;
      }
      signal.wait().await;
    }
  }
}

/// Type alias for `Arc`-based wait group using `CriticalSectionRawMutex`
///
/// Provides interrupt-safe wait group synchronization for embedded contexts.
pub type ArcLocalWaitGroup = CoreWaitGroup<ArcWaitGroupBackend<CriticalSectionRawMutex>>;

/// Alias for `ArcLocalWaitGroup` for consistency
///
/// Uses critical section signal backend.
pub type ArcCsWaitGroup = ArcLocalWaitGroup;

