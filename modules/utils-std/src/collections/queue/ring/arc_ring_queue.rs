use crate::{collections::queue::MutexRingBufferStorage, sync::ArcShared};

type ArcRingStorage<E> = ArcShared<RingStorageBackend<ArcShared<MutexRingBufferStorage<E>>>>;
use cellex_utils_core_rs::{
  collections::queue::{
    ring::{RingQueue, RingStorageBackend, DEFAULT_CAPACITY},
    traits::{QueueBase, QueueReader, QueueRw, QueueWriter},
    QueueSize,
  },
  v2::collections::queue::backend::QueueError,
};

#[cfg(test)]
mod tests;

/// Ring buffer-based circular queue
///
/// A ring buffer queue with fixed capacity or dynamic expansion capability.
/// Can be safely accessed from multiple threads using `Arc`-based shared ownership.
#[derive(Debug, Clone)]
pub struct ArcRingQueue<E> {
  inner: RingQueue<ArcRingStorage<E>, E>,
}

impl<E> ArcRingQueue<E> {
  /// Creates a new ring queue with the specified capacity
  #[must_use]
  pub fn new(capacity: usize) -> Self {
    let storage = ArcShared::new(MutexRingBufferStorage::with_capacity(capacity));
    let backend: ArcRingStorage<E> = ArcShared::new(RingStorageBackend::new(storage));
    Self { inner: RingQueue::new(backend) }
  }

  /// Sets dynamic expansion mode and returns the queue (builder pattern)
  #[must_use]
  pub fn with_dynamic(mut self, dynamic: bool) -> Self {
    self.inner = self.inner.with_dynamic(dynamic);
    self
  }

  /// Sets dynamic expansion mode
  pub fn set_dynamic(&self, dynamic: bool) {
    self.inner.set_dynamic(dynamic);
  }
}

impl<E> Default for ArcRingQueue<E> {
  fn default() -> Self {
    Self::new(DEFAULT_CAPACITY)
  }
}

impl<E> QueueBase<E> for ArcRingQueue<E> {
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E> QueueWriter<E> for ArcRingQueue<E> {
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E> QueueReader<E> for ArcRingQueue<E> {
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E> QueueRw<E> for ArcRingQueue<E> {
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
