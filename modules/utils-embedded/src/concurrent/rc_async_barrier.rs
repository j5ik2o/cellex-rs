use alloc::{boxed::Box, rc::Rc};
use core::cell::RefCell;

use async_trait::async_trait;
use cellex_utils_core_rs::{AsyncBarrier as CoreAsyncBarrier, AsyncBarrierBackend};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};

#[cfg(test)]
mod tests;

/// Backend for `Rc`-based asynchronous barrier implementation.
///
/// Provides functionality for multiple tasks to wait and resume at a specific synchronization point
/// (barrier) in `no_std` environments. Internally uses `RefCell` and Embassy's `Signal` to achieve
/// single-threaded synchronization.
///
/// # Features
///
/// - Reference counting via `Rc` (single-threaded only)
/// - Lightweight synchronization via Embassy's `NoopRawMutex`
/// - All tasks are released simultaneously when count reaches zero
#[derive(Clone)]
pub struct RcAsyncBarrierBackend {
  remaining: Rc<RefCell<usize>>,
  initial:   usize,
  signal:    Rc<Signal<NoopRawMutex, ()>>,
}

#[async_trait(?Send)]
impl AsyncBarrierBackend for RcAsyncBarrierBackend {
  /// Creates a new barrier backend with the specified count.
  ///
  /// # Panics
  ///
  /// Panics if `count` is 0.
  fn new(count: usize) -> Self {
    assert!(count > 0, "AsyncBarrier must have positive count");
    Self { remaining: Rc::new(RefCell::new(count)), initial: count, signal: Rc::new(Signal::new()) }
  }

  /// Waits at the barrier.
  ///
  /// Blocks until all participants (`count` tasks) call `wait()`.
  /// When the last task reaches the barrier, all tasks are released simultaneously.
  ///
  /// # Panics
  ///
  /// Panics if `wait` is called more than `count` times.
  async fn wait(&self) {
    let remaining = self.remaining.clone();
    let signal = self.signal.clone();
    let initial = self.initial;
    {
      let mut rem = remaining.borrow_mut();
      assert!(*rem > 0, "AsyncBarrier::wait called more times than count");
      *rem -= 1;
      if *rem == 0 {
        *rem = initial;
        signal.signal(());
        return;
      }
    }

    loop {
      signal.wait().await;
      if *remaining.borrow() == initial {
        break;
      }
    }
  }
}

/// Type alias for `Rc`-based asynchronous barrier.
///
/// Asynchronous barrier implementation usable in `no_std` environments.
/// Provides functionality for multiple tasks to wait at synchronization points until all are ready.
pub type AsyncBarrier = CoreAsyncBarrier<RcAsyncBarrierBackend>;
