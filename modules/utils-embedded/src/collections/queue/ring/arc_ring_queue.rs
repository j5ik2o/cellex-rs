use cellex_utils_core_rs::{
  QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter, RingBuffer, RingQueue, RingStorageBackend,
  DEFAULT_CAPACITY,
};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};

use crate::sync::{ArcShared, ArcStateCell};

/// `Arc`-based ring queue with configurable mutex backend
///
/// This queue provides FIFO semantics using a circular buffer, with `Arc` for
/// thread-safe reference counting. The mutex backend is configurable via the `RM`
/// type parameter.
///
/// # Type Parameters
///
/// * `E` - Element type stored in the queue
/// * `RM` - Raw mutex type (defaults to `NoopRawMutex`)
#[derive(Debug)]
pub struct ArcRingQueue<E, RM = NoopRawMutex>
where
  RM: RawMutex, {
  inner: RingQueue<ArcShared<RingStorageBackend<ArcShared<ArcStateCell<RingBuffer<E>, RM>>>>, E>,
}

/// Type alias for `ArcRingQueue` using `NoopRawMutex`
///
/// Suitable for single-threaded or interrupt-free contexts where no locking is required.
pub type ArcLocalRingQueue<E> = ArcRingQueue<E, NoopRawMutex>;

impl<E, RM> ArcRingQueue<E, RM>
where
  RM: RawMutex,
{
  /// Creates a new ring queue with the specified capacity
  ///
  /// # Arguments
  ///
  /// * `capacity` - Initial capacity of the queue
  pub fn new(capacity: usize) -> Self {
    let storage = ArcShared::new(ArcStateCell::new(RingBuffer::new(capacity)));
    let backend = ArcShared::new(RingStorageBackend::new(storage));
    Self { inner: RingQueue::new(backend) }
  }

  /// Sets the dynamic expansion mode and returns self (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, queue expands dynamically; if `false`, capacity is fixed
  pub fn with_dynamic(mut self, dynamic: bool) -> Self {
    self.inner = self.inner.with_dynamic(dynamic);
    self
  }

  /// Sets the dynamic expansion mode
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, queue expands dynamically when full
  pub fn set_dynamic(&self, dynamic: bool) {
    self.inner.set_dynamic(dynamic);
  }
}

impl<E, RM> QueueBase<E> for ArcRingQueue<E, RM>
where
  RM: RawMutex,
{
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E, RM> QueueWriter<E> for ArcRingQueue<E, RM>
where
  RM: RawMutex,
{
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E, RM> QueueReader<E> for ArcRingQueue<E, RM>
where
  RM: RawMutex,
{
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E, RM> QueueRw<E> for ArcRingQueue<E, RM>
where
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

impl<E, RM> Default for ArcRingQueue<E, RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self::new(DEFAULT_CAPACITY)
  }
}

impl<E, RM> Clone for ArcRingQueue<E, RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}
