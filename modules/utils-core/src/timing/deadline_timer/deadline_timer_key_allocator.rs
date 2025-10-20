#[cfg(not(target_has_atomic = "ptr"))]
use core::cell::Cell;
#[cfg(target_has_atomic = "ptr")]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(not(target_has_atomic = "ptr"))]
use critical_section::with;

use super::deadline_timer_key::DeadlineTimerKey;

/// Allocator for generating [`DeadlineTimerKey`] values.
#[derive(Debug)]
pub struct DeadlineTimerKeyAllocator {
  #[cfg(target_has_atomic = "ptr")]
  counter: AtomicUsize,
  #[cfg(not(target_has_atomic = "ptr"))]
  counter: Cell<usize>,
}

impl DeadlineTimerKeyAllocator {
  /// Creates a new allocator.
  #[must_use]
  #[inline]
  pub const fn new() -> Self {
    #[cfg(target_has_atomic = "ptr")]
    {
      Self { counter: AtomicUsize::new(1) }
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    {
      Self { counter: Cell::new(1) }
    }
  }

  /// Issues a new unique key.
  #[inline]
  pub fn allocate(&self) -> DeadlineTimerKey {
    #[cfg(target_has_atomic = "ptr")]
    {
      let next = self.counter.fetch_add(1, Ordering::Relaxed) as u64;
      let raw = if next == 0 { 1 } else { next };
      DeadlineTimerKey::from_raw(raw)
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    {
      let issued = with(|_| {
        let current = self.counter.get();
        let next = current.wrapping_add(1);
        let stored = if next == 0 { 1 } else { next };
        self.counter.set(stored);
        if current == 0 {
          1
        } else {
          current
        }
      });
      DeadlineTimerKey::from_raw(issued as u64)
    }
  }

  /// Checks the next key to be issued (for testing purposes).
  #[inline]
  pub fn peek(&self) -> DeadlineTimerKey {
    #[cfg(target_has_atomic = "ptr")]
    {
      DeadlineTimerKey::from_raw(self.counter.load(Ordering::Relaxed) as u64)
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    {
      with(|_| DeadlineTimerKey::from_raw(self.counter.get() as u64))
    }
  }
}

impl Default for DeadlineTimerKeyAllocator {
  fn default() -> Self {
    Self::new()
  }
}
