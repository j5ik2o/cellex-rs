use std::{
  fmt,
  sync::{Arc, Mutex},
};

use cellex_utils_core_rs::{
  Element, MpscBackend, MpscBuffer, MpscQueue, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter,
  RingBufferBackend,
};

use crate::{collections::queue::mpsc::TokioBoundedMpscBackend, sync::ArcShared};

#[cfg(test)]
mod tests;

/// Bounded multi-producer, single-consumer (MPSC) queue
///
/// A queue that can be safely accessed from multiple threads using `Arc`-based shared ownership.
/// By default, it uses a Tokio channel backend, but a ring buffer backend can also be selected.
#[derive(Clone)]
pub struct ArcMpscBoundedQueue<E> {
  inner: MpscQueue<ArcShared<dyn MpscBackend<E> + Send + Sync>, E>,
}

impl<E> fmt::Debug for ArcMpscBoundedQueue<E> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ArcMpscBoundedQueue").finish()
  }
}

impl<E> ArcMpscBoundedQueue<E>
where
  E: Element,
{
  /// Creates a new queue with the specified capacity (using Tokio backend)
  ///
  /// # Arguments
  ///
  /// * `capacity` - Maximum capacity of the queue
  ///
  /// # Returns
  ///
  /// A new queue instance using the Tokio channel backend
  #[must_use]
  pub fn new(capacity: usize) -> Self {
    Self::with_tokio(capacity)
  }

  /// Creates a queue using the Tokio channel backend
  ///
  /// # Arguments
  ///
  /// * `capacity` - Maximum capacity of the queue
  ///
  /// # Returns
  ///
  /// A new queue instance using the Tokio channel backend
  #[must_use]
  pub fn with_tokio(capacity: usize) -> Self {
    Self::from_backend(TokioBoundedMpscBackend::new(capacity))
  }

  /// Creates a queue using the ring buffer backend
  ///
  /// # Arguments
  ///
  /// * `capacity` - Maximum capacity of the queue
  ///
  /// # Returns
  ///
  /// A new queue instance using the ring buffer backend
  #[must_use]
  pub fn with_ring_buffer(capacity: usize) -> Self {
    let backend = RingBufferBackend::new(Mutex::new(MpscBuffer::new(Some(capacity))));
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

impl<E: Element> QueueBase<E> for ArcMpscBoundedQueue<E> {
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E: Element> QueueWriter<E> for ArcMpscBoundedQueue<E> {
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E: Element> QueueReader<E> for ArcMpscBoundedQueue<E> {
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E: Element> QueueRw<E> for ArcMpscBoundedQueue<E> {
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
