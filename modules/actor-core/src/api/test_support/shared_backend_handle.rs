use core::{cell::RefCell, fmt};

use cellex_utils_core_rs::{ArcShared, MpscBuffer, MpscHandle, RingBufferBackend, Shared};

/// Shared handle around the ring-buffer backend used to simulate mailbox queues in tests.
pub struct SharedBackendHandle<T>(ArcShared<RingBufferBackend<RefCell<MpscBuffer<T>>>>);

impl<T> SharedBackendHandle<T> {
  /// Creates a new handle with an optional custom queue capacity.
  pub fn new(capacity: Option<usize>) -> Self {
    let buffer = RefCell::new(MpscBuffer::new(capacity));
    let backend = RingBufferBackend::new(buffer);
    Self(ArcShared::new(backend))
  }
}

impl<T> Clone for SharedBackendHandle<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> core::ops::Deref for SharedBackendHandle<T> {
  type Target = RingBufferBackend<RefCell<MpscBuffer<T>>>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> fmt::Debug for SharedBackendHandle<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SharedBackendHandle").finish()
  }
}

impl<T> Shared<RingBufferBackend<RefCell<MpscBuffer<T>>>> for SharedBackendHandle<T> {
  fn try_unwrap(self) -> Result<RingBufferBackend<RefCell<MpscBuffer<T>>>, Self>
  where
    RingBufferBackend<RefCell<MpscBuffer<T>>>: Sized, {
    match self.0.try_unwrap() {
      | Ok(inner) => Ok(inner),
      | Err(shared) => Err(Self(shared)),
    }
  }
}

impl<T> MpscHandle<T> for SharedBackendHandle<T> {
  type Backend = RingBufferBackend<RefCell<MpscBuffer<T>>>;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}
