#[cfg(test)]
mod tests;

use cellex_utils_core_rs::{
  collections::{
    queue::{
      mpsc::{MpscBuffer, MpscQueue, RingBufferBackend},
      traits::{QueueBase, QueueReader, QueueRw, QueueWriter},
      QueueSize,
    },
    Element,
  },
  v2::collections::queue::backend::QueueError,
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex};

use crate::sync::arc::{ArcShared, ArcStateCell};

/// `Arc`-based unbounded MPSC queue with configurable mutex backend
///
/// This queue provides Multi-Producer-Single-Consumer semantics with dynamic capacity,
/// using `Arc` for thread-safe reference counting. The mutex backend is configurable
/// via the `RM` type parameter, allowing selection between `NoopRawMutex` for
/// single-threaded or interrupt-free contexts, and `CriticalSectionRawMutex` for
/// interrupt-safe critical sections.
///
/// # Type Parameters
///
/// * `E` - Element type stored in the queue
/// * `RM` - Raw mutex type (defaults to `NoopRawMutex`)
#[derive(Debug)]
#[deprecated(
  since = "0.0.1",
  note = "Use cellex_utils_core_rs::v2::collections::AsyncQueue with SpinAsyncMutexCritical instead."
)]
pub struct ArcMpscUnboundedQueue<E, RM = NoopRawMutex>
where
  RM: RawMutex, {
  inner: MpscQueue<ArcShared<RingBufferBackend<ArcStateCell<MpscBuffer<E>, RM>>>, E>,
}

impl<E, RM> Clone for ArcMpscUnboundedQueue<E, RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

/// Type alias for `ArcMpscUnboundedQueue` using `NoopRawMutex`
///
/// Suitable for single-threaded or interrupt-free contexts where no locking is required.
pub type ArcLocalMpscUnboundedQueue<E> = ArcMpscUnboundedQueue<E, NoopRawMutex>;

/// Type alias for `ArcMpscUnboundedQueue` using `CriticalSectionRawMutex`
///
/// Provides interrupt-safe critical section protection for multi-threaded embedded contexts.
pub type ArcCsMpscUnboundedQueue<E> = ArcMpscUnboundedQueue<E, CriticalSectionRawMutex>;

impl<E, RM> ArcMpscUnboundedQueue<E, RM>
where
  RM: RawMutex,
{
  /// Creates a new unbounded MPSC queue
  ///
  /// The queue will dynamically grow as needed to accommodate elements.
  pub fn new() -> Self {
    let storage = ArcShared::new(RingBufferBackend::new(ArcStateCell::new(MpscBuffer::new(None))));
    Self { inner: MpscQueue::new(storage) }
  }
}

impl<E, RM> QueueBase<E> for ArcMpscUnboundedQueue<E, RM>
where
  E: Element,
  RM: RawMutex,
{
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E, RM> QueueWriter<E> for ArcMpscUnboundedQueue<E, RM>
where
  E: Element,
  RM: RawMutex,
{
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E, RM> QueueReader<E> for ArcMpscUnboundedQueue<E, RM>
where
  E: Element,
  RM: RawMutex,
{
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E, RM> QueueRw<E> for ArcMpscUnboundedQueue<E, RM>
where
  E: Element,
  RM: RawMutex,
{
  fn offer(&self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer(element)
  }

  fn poll(&self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll()
  }

  fn clean_up(&self) {
    self.inner.clean_up();
  }
}
