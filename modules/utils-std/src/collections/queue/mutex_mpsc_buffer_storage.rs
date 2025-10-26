use std::sync::Mutex;

use cellex_utils_core_rs::collections::queue::{mpsc::MpscBuffer, ring::RingBufferStorage};

use crate::sync::ArcShared;

/// Mutex-backed storage for MPSC ring buffers in std environments.
pub struct MutexMpscBufferStorage<T> {
  inner: Mutex<MpscBuffer<T>>,
}

impl<T> MutexMpscBufferStorage<T> {
  #[must_use]
  pub fn with_capacity(capacity: Option<usize>) -> Self {
    Self { inner: Mutex::new(MpscBuffer::new(capacity)) }
  }
}

impl<T> RingBufferStorage<T> for MutexMpscBufferStorage<T> {
  fn with_read<R>(&self, f: impl FnOnce(&MpscBuffer<T>) -> R) -> R {
    let guard = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&guard)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut MpscBuffer<T>) -> R) -> R {
    let mut guard = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
  }
}

impl<T> RingBufferStorage<T> for ArcShared<MutexMpscBufferStorage<T>> {
  fn with_read<R>(&self, f: impl FnOnce(&MpscBuffer<T>) -> R) -> R {
    (**self).with_read(f)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut MpscBuffer<T>) -> R) -> R {
    (**self).with_write(f)
  }
}
