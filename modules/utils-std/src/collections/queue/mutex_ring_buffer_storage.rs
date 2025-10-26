use std::sync::Mutex;

use cellex_utils_core_rs::collections::queue::{ring::RingBuffer, traits::QueueStorage};

use crate::sync::ArcShared;

/// Mutex-backed storage for ring buffers used in std environments.
pub struct MutexRingBufferStorage<E> {
  inner: Mutex<RingBuffer<E>>,
}

impl<E> MutexRingBufferStorage<E> {
  #[must_use]
  pub fn with_capacity(capacity: usize) -> Self {
    Self { inner: Mutex::new(RingBuffer::new(capacity)) }
  }
}

impl<E> QueueStorage<E> for MutexRingBufferStorage<E> {
  fn with_read<R>(&self, f: impl FnOnce(&RingBuffer<E>) -> R) -> R {
    let guard = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&guard)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut RingBuffer<E>) -> R) -> R {
    let mut guard = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
  }
}

impl<E> QueueStorage<E> for ArcShared<MutexRingBufferStorage<E>> {
  fn with_read<R>(&self, f: impl FnOnce(&RingBuffer<E>) -> R) -> R {
    (**self).with_read(f)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut RingBuffer<E>) -> R) -> R {
    (**self).with_write(f)
  }
}
