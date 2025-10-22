mod arc_shared_ring_queue;
mod backend;
mod buffer;
mod queue;

#[cfg(feature = "alloc")]
use core::{cell::RefCell, ops::Deref};

#[allow(unused_imports)]
pub use arc_shared_ring_queue::ArcSharedRingQueue;
pub use backend::{RingBackend, RingHandle, RingStorageBackend};
pub use buffer::{RingBuffer, DEFAULT_CAPACITY};
pub use queue::RingQueue;

#[cfg(feature = "alloc")]
use crate::{collections::queue::traits::QueueHandle, sync::RcShared};

#[cfg(feature = "alloc")]
impl<E> QueueHandle<E> for RcShared<RefCell<crate::collections::queue::RingBuffer<E>>> {
  type Storage = RefCell<crate::collections::queue::RingBuffer<E>>;

  fn storage(&self) -> &Self::Storage {
    self.deref()
  }
}

#[cfg(feature = "alloc")]
impl<E> crate::collections::queue::ring::backend::RingHandle<E>
  for RcShared<
    crate::collections::queue::RingStorageBackend<RcShared<RefCell<crate::collections::queue::RingBuffer<E>>>>,
  >
{
  type Backend =
    crate::collections::queue::RingStorageBackend<RcShared<RefCell<crate::collections::queue::RingBuffer<E>>>>;

  fn backend(&self) -> &Self::Backend {
    self.deref()
  }
}
