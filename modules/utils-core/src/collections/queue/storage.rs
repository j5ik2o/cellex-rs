mod queue_storage;

pub use queue_storage::QueueStorage;

use super::mpsc::MpscBuffer;

/// Ring buffer-based storage abstraction trait
///
/// Provides storage abstraction shared by [`crate::collections::queue::mpsc::RingBufferBackend`]
/// implementations. This trait offers an interface for uniformly handling read and write access to
/// MPSC buffers.
///
/// # Type Parameters
///
/// * `T` - Type of elements stored in the buffer
pub trait RingBufferStorage<T> {
  /// Executes a closure using an immutable reference to the MPSC buffer
  ///
  /// # Arguments
  ///
  /// * `f` - Closure receiving an immutable reference to the MPSC buffer
  ///
  /// # Returns
  ///
  /// Result of executing the closure
  fn with_read<R>(&self, f: impl FnOnce(&MpscBuffer<T>) -> R) -> R;

  /// Executes a closure using a mutable reference to the MPSC buffer
  ///
  /// # Arguments
  ///
  /// * `f` - Closure receiving a mutable reference to the MPSC buffer
  ///
  /// # Returns
  ///
  /// Result of executing the closure
  fn with_write<R>(&self, f: impl FnOnce(&mut MpscBuffer<T>) -> R) -> R;
}

#[cfg(feature = "alloc")]
mod mpsc_alloc_impls {
  use core::cell::RefCell;

  use super::{MpscBuffer, RingBufferStorage};

  impl<T> RingBufferStorage<T> for RefCell<MpscBuffer<T>> {
    fn with_read<R>(&self, f: impl FnOnce(&MpscBuffer<T>) -> R) -> R {
      let guard = self.borrow();
      f(&guard)
    }

    fn with_write<R>(&self, f: impl FnOnce(&mut MpscBuffer<T>) -> R) -> R {
      let mut guard = self.borrow_mut();
      f(&mut guard)
    }
  }
}

#[cfg(all(feature = "alloc", feature = "std"))]
mod mpsc_std_impls {
  use std::sync::Mutex;

  use super::{MpscBuffer, RingBufferStorage};

  impl<T> RingBufferStorage<T> for Mutex<MpscBuffer<T>> {
    fn with_read<R>(&self, f: impl FnOnce(&MpscBuffer<T>) -> R) -> R {
      let guard = self.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      f(&guard)
    }

    fn with_write<R>(&self, f: impl FnOnce(&mut MpscBuffer<T>) -> R) -> R {
      let mut guard = self.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      f(&mut guard)
    }
  }
}
