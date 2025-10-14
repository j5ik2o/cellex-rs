use core::cell::RefCell;

use cellex_utils_core_rs::{
  Element, MpscBuffer, MpscQueue, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter,
  RingBufferBackend,
};

use crate::sync::RcShared;

/// `Rc`-based unbounded MPSC (Multiple Producer, Single Consumer) queue
///
/// This queue is an MPSC queue without capacity limits, usable in `no_std` environments.
/// It provides reference-counted shared ownership using `Rc` and `RefCell`.
///
/// # Features
///
/// - **Unbounded**: Can add elements without capacity limits (as memory permits)
/// - **MPSC**: Supports multiple producers and a single consumer
/// - **no_std Support**: Does not require the standard library
/// - **Cloneable**: Multiple handles can be created via `clone()`
///
/// # Performance Characteristics
///
/// - `offer`: O(1) (amortized), O(n) when resizing
/// - `poll`: O(1)
/// - Memory usage: O(n) (proportional to number of elements)
///
/// # Memory Considerations
///
/// Since this queue has no capacity limit, it may cause memory exhaustion.
/// It is recommended to implement appropriate backpressure control in production environments.
///
/// # Examples
///
/// ```
/// use cellex_utils_embedded_rs::RcMpscUnboundedQueue;
/// use cellex_utils_core_rs::QueueRw;
///
/// let queue = RcMpscUnboundedQueue::new();
/// queue.offer(1).unwrap();
/// queue.offer(2).unwrap();
/// assert_eq!(queue.poll().unwrap(), Some(1));
/// assert_eq!(queue.poll().unwrap(), Some(2));
/// ```
#[derive(Debug, Clone)]
pub struct RcMpscUnboundedQueue<E> {
  inner: MpscQueue<RcShared<RingBufferBackend<RefCell<MpscBuffer<E>>>>, E>,
}

impl<E> RcMpscUnboundedQueue<E> {
  /// Creates a new unbounded MPSC queue
  ///
  /// This queue has no capacity limit and expands dynamically.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::RcMpscUnboundedQueue;
  ///
  /// let queue: RcMpscUnboundedQueue<String> = RcMpscUnboundedQueue::new();
  /// ```
  pub fn new() -> Self {
    let storage = RcShared::new(RingBufferBackend::new(RefCell::new(MpscBuffer::new(None))));
    Self {
      inner: MpscQueue::new(storage),
    }
  }
}

impl<E> Default for RcMpscUnboundedQueue<E> {
  fn default() -> Self {
    Self::new()
  }
}

impl<E: Element> QueueBase<E> for RcMpscUnboundedQueue<E> {
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E: Element> QueueWriter<E> for RcMpscUnboundedQueue<E> {
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E: Element> QueueReader<E> for RcMpscUnboundedQueue<E> {
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E: Element> QueueRw<E> for RcMpscUnboundedQueue<E> {
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
