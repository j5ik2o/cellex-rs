use std::sync::Mutex;

use crate::sync::ArcShared;

type ArcRingStorage<E> = ArcShared<RingStorageBackend<ArcShared<Mutex<RingBuffer<E>>>>>;
use cellex_utils_core_rs::{
  QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter, RingBuffer, RingQueue, RingStorageBackend,
  DEFAULT_CAPACITY,
};

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
  ///
  /// # Arguments
  ///
  /// * `capacity` - Initial capacity of the ring buffer
  ///
  /// # Returns
  ///
  /// A new ring queue instance
  pub fn new(capacity: usize) -> Self {
    let storage = ArcShared::new(Mutex::new(RingBuffer::new(capacity)));
    let backend: ArcRingStorage<E> = ArcShared::new(RingStorageBackend::new(storage));
    Self { inner: RingQueue::new(backend) }
  }

  /// Sets dynamic expansion mode and returns the queue (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, automatically expands when capacity is exceeded
  ///
  /// # Returns
  ///
  /// The queue instance with the configuration applied (self)
  pub fn with_dynamic(mut self, dynamic: bool) -> Self {
    self.inner = self.inner.with_dynamic(dynamic);
    self
  }

  /// Sets dynamic expansion mode
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, automatically expands when capacity is exceeded
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

#[cfg(test)]
mod tests;
