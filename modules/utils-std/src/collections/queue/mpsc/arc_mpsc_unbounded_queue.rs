use std::{
  fmt,
  sync::{Arc, Mutex},
};

use cellex_utils_core_rs::{
  Element, MpscBackend, MpscBuffer, MpscQueue, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter,
  RingBufferBackend,
};

use crate::{collections::queue::mpsc::TokioUnboundedMpscBackend, sync::ArcShared};

#[cfg(test)]
mod tests;

/// Unbounded multi-producer, single-consumer (MPSC) queue
///
/// An unbounded queue that can be safely accessed from multiple threads using `Arc`-based shared
/// ownership. By default, it uses a Tokio unbounded channel backend, but a ring buffer backend can
/// also be selected.
#[derive(Clone)]
pub struct ArcMpscUnboundedQueue<E> {
  inner: MpscQueue<ArcShared<dyn MpscBackend<E> + Send + Sync>, E>,
}

impl<E> fmt::Debug for ArcMpscUnboundedQueue<E> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ArcMpscUnboundedQueue").finish()
  }
}

impl<E> ArcMpscUnboundedQueue<E>
where
  E: Element,
{
  /// Creates a new unbounded queue (using Tokio unbounded backend)
  ///
  /// # Returns
  ///
  /// A new queue instance using the Tokio unbounded channel backend
  pub fn new() -> Self {
    Self::with_tokio()
  }

  /// Creates a queue using the Tokio unbounded channel backend
  ///
  /// # Returns
  ///
  /// A new queue instance using the Tokio unbounded channel backend
  pub fn with_tokio() -> Self {
    Self::from_backend(TokioUnboundedMpscBackend::new())
  }

  /// Creates an unbounded queue using the ring buffer backend
  ///
  /// # Returns
  ///
  /// A new queue instance using the ring buffer backend
  pub fn with_ring_buffer() -> Self {
    let backend = RingBufferBackend::new(Mutex::new(MpscBuffer::new(None)));
    Self::from_backend(backend)
  }

  fn from_backend<B>(backend: B) -> Self
  where
    B: MpscBackend<E> + Send + Sync + 'static, {
    let arc_backend: Arc<dyn MpscBackend<E> + Send + Sync> = Arc::new(backend);
    let storage = ArcShared::from_arc(arc_backend);
    Self { inner: MpscQueue::new(storage) }
  }
}

impl<E: Element> QueueBase<E> for ArcMpscUnboundedQueue<E> {
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E: Element> QueueWriter<E> for ArcMpscUnboundedQueue<E> {
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E: Element> QueueReader<E> for ArcMpscUnboundedQueue<E> {
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E: Element> QueueRw<E> for ArcMpscUnboundedQueue<E> {
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

impl<E> Default for ArcMpscUnboundedQueue<E>
where
  E: Element,
{
  fn default() -> Self {
    Self::new()
  }
}
