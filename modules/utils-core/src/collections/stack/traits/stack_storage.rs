#[cfg(feature = "alloc")]
use core::cell::RefCell;
#[cfg(all(feature = "alloc", feature = "std"))]
use std::sync::Mutex;

use crate::collections::stack::buffer::StackBuffer;

/// Abstraction for storage used by stack backends.
pub trait StackStorage<T> {
  /// Executes a closure with read-only access.
  fn with_read<R>(&self, f: impl FnOnce(&StackBuffer<T>) -> R) -> R;

  /// Executes a closure with writable access.
  fn with_write<R>(&self, f: impl FnOnce(&mut StackBuffer<T>) -> R) -> R;
}

#[cfg(feature = "alloc")]
impl<T> StackStorage<T> for RefCell<StackBuffer<T>> {
  fn with_read<R>(&self, f: impl FnOnce(&StackBuffer<T>) -> R) -> R {
    f(&self.borrow())
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut StackBuffer<T>) -> R) -> R {
    f(&mut self.borrow_mut())
  }
}

#[cfg(all(feature = "alloc", feature = "std"))]
impl<T> StackStorage<T> for Mutex<StackBuffer<T>> {
  #[allow(clippy::expect_used)]
  fn with_read<R>(&self, f: impl FnOnce(&StackBuffer<T>) -> R) -> R {
    let guard = self.lock().expect("mutex poisoned");
    f(&guard)
  }

  #[allow(clippy::expect_used)]
  fn with_write<R>(&self, f: impl FnOnce(&mut StackBuffer<T>) -> R) -> R {
    let mut guard = self.lock().expect("mutex poisoned");
    f(&mut guard)
  }
}
