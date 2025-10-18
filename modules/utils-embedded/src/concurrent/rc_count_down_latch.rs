use alloc::{boxed::Box, rc::Rc};

use cellex_utils_core_rs::{async_trait, CountDownLatch as CoreCountDownLatch, CountDownLatchBackend};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex, signal::Signal};

/// Backend for `Rc`-based countdown latch implementation.
///
/// Provides a synchronization mechanism that waits for multiple tasks to complete in `no_std`
/// environments. When the count decrements from the specified value to 0, all waiting tasks are
/// released.
///
/// # Features
///
/// - Reference counting via `Rc` (single-threaded only)
/// - Asynchronous synchronization via Embassy's `Mutex` and `Signal`
/// - One-way countdown (count only decreases)
///
/// # Usage Examples
///
/// ```ignore
/// let latch = CountDownLatch::new(2);
/// let clone = latch.clone();
///
/// // Worker task
/// async move {
///   // Perform work
///   clone.count_down().await;
///   // Perform more work
///   clone.count_down().await;
/// };
///
/// // Wait until count reaches 0
/// latch.wait().await;
/// ```
#[derive(Clone)]
pub struct RcCountDownLatchBackend {
  count:  Rc<Mutex<NoopRawMutex, usize>>,
  signal: Rc<Signal<NoopRawMutex, ()>>,
}

#[async_trait(?Send)]
impl CountDownLatchBackend for RcCountDownLatchBackend {
  /// Creates a new latch backend with the specified count.
  ///
  /// # Arguments
  ///
  /// * `count` - Initial count value (0 is allowed)
  fn new(count: usize) -> Self {
    Self { count: Rc::new(Mutex::new(count)), signal: Rc::new(Signal::new()) }
  }

  /// Decrements the count by 1.
  ///
  /// When the count reaches 0, all waiting tasks receive a signal and are released.
  ///
  /// # Panics
  ///
  /// Panics if called when the count is already 0.
  async fn count_down(&self) {
    let count = self.count.clone();
    let signal = self.signal.clone();
    let mut guard = count.lock().await;
    assert!(*guard > 0, "CountDownLatch::count_down called too many times");
    *guard -= 1;
    if *guard == 0 {
      signal.signal(());
    }
  }

  /// Waits until the count reaches 0.
  ///
  /// Returns immediately if the count is already 0.
  async fn wait(&self) {
    let count = self.count.clone();
    let signal = self.signal.clone();
    loop {
      {
        let guard = count.lock().await;
        if *guard == 0 {
          return;
        }
      }
      signal.wait().await;
    }
  }
}

/// Type alias for `Rc`-based countdown latch.
///
/// Countdown latch implementation usable in `no_std` environments.
/// Provides functionality to wait for multiple tasks to complete until the count reaches 0.
pub type CountDownLatch = CoreCountDownLatch<RcCountDownLatchBackend>;

#[cfg(test)]
mod tests;
