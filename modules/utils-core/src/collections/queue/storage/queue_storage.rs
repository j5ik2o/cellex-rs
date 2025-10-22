#[cfg(feature = "alloc")]
use core::cell::RefCell;

use crate::collections::queue::ring::RingBuffer;

/// Queue storage abstraction trait.
pub trait QueueStorage<E> {
  /// Executes the provided closure with an immutable reference to the ring buffer.
  fn with_read<R>(&self, f: impl FnOnce(&RingBuffer<E>) -> R) -> R;

  /// Executes the provided closure with a mutable reference to the ring buffer.
  fn with_write<R>(&self, f: impl FnOnce(&mut RingBuffer<E>) -> R) -> R;
}

#[cfg(feature = "alloc")]
impl<E> QueueStorage<E> for RefCell<RingBuffer<E>> {
  fn with_read<R>(&self, f: impl FnOnce(&RingBuffer<E>) -> R) -> R {
    let guard = self.borrow();
    f(&guard)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut RingBuffer<E>) -> R) -> R {
    let mut guard = self.borrow_mut();
    f(&mut guard)
  }
}
