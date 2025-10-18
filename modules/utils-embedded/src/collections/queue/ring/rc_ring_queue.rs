use core::cell::RefCell;

use cellex_utils_core_rs::{
  QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter, RingBuffer, RingQueue, RingStorageBackend,
  DEFAULT_CAPACITY,
};

use crate::sync::RcShared;

/// `Rc`-based ring buffer storage type alias
///
/// Reference-counted ring buffer storage using `RcShared` and `RefCell`.
type RcRingStorage<E> = RcShared<RingStorageBackend<RcShared<RefCell<RingBuffer<E>>>>>;

/// `Rc`-based ring buffer queue
///
/// This queue is a FIFO (First-In-First-Out) queue using a circular buffer,
/// available in `no_std` environments. It provides reference-counted shared ownership
/// using `Rc` and `RefCell`.
///
/// # Features
///
/// - **Ring buffer**: Efficient circular buffer implementation
/// - **Dynamic/static modes**: Choose between dynamic expansion or fixed capacity
/// - **no_std support**: Does not require the standard library
/// - **Cloneable**: Multiple handles can be created via `clone()`
///
/// # Performance characteristics
///
/// - `offer`: O(1) (within capacity), O(n) during resize
/// - `poll`: O(1)
/// - Memory usage: O(capacity)
///
/// # Modes
///
/// - **Dynamic mode**: Automatically expands when capacity is insufficient (default)
/// - **Static mode**: Capacity limits are strictly enforced, returns `QueueError::Full` when full
///
/// # Examples
///
/// ```
/// use cellex_utils_core_rs::QueueRw;
/// use cellex_utils_embedded_rs::RcRingQueue;
///
/// // Create a dynamic ring queue with capacity 10
/// let queue: RcRingQueue<i32> = RcRingQueue::new(10);
/// queue.offer(1).unwrap();
/// queue.offer(2).unwrap();
/// assert_eq!(queue.poll().unwrap(), Some(1));
///
/// // Create a fixed-capacity ring queue
/// let static_queue: RcRingQueue<i32> = RcRingQueue::new(5).with_dynamic(false);
/// ```
#[derive(Debug, Clone)]
pub struct RcRingQueue<E> {
  inner: RingQueue<RcRingStorage<E>, E>,
}

impl<E> RcRingQueue<E> {
  /// Creates a new ring buffer queue with the specified capacity
  ///
  /// By default, created in dynamic expansion mode.
  ///
  /// # Arguments
  ///
  /// * `capacity` - Initial capacity of the queue
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::RcRingQueue;
  ///
  /// let queue: RcRingQueue<String> = RcRingQueue::new(100);
  /// ```
  pub fn new(capacity: usize) -> Self {
    let storage = RcShared::new(RefCell::new(RingBuffer::new(capacity)));
    let backend: RcRingStorage<E> = RcShared::new(RingStorageBackend::new(storage));
    Self { inner: RingQueue::new(backend) }
  }

  /// Sets the dynamic expansion mode and returns self (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, automatically expands when capacity is insufficient. If `false`,
  ///   capacity limits are strictly enforced.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::RcRingQueue;
  ///
  /// let queue: RcRingQueue<i32> = RcRingQueue::new(10).with_dynamic(false); // Fixed capacity mode
  /// ```
  pub fn with_dynamic(mut self, dynamic: bool) -> Self {
    self.inner = self.inner.with_dynamic(dynamic);
    self
  }

  /// Sets the dynamic expansion mode of the queue
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, automatically expands when capacity is insufficient
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::RcRingQueue;
  ///
  /// let queue: RcRingQueue<i32> = RcRingQueue::new(10);
  /// queue.set_dynamic(false); // Change to fixed capacity mode
  /// ```
  pub fn set_dynamic(&self, dynamic: bool) {
    self.inner.set_dynamic(dynamic);
  }
}

impl<E> Default for RcRingQueue<E> {
  fn default() -> Self {
    Self::new(DEFAULT_CAPACITY)
  }
}

impl<E> QueueBase<E> for RcRingQueue<E> {
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E> QueueWriter<E> for RcRingQueue<E> {
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E> QueueReader<E> for RcRingQueue<E> {
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E> QueueRw<E> for RcRingQueue<E> {
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
