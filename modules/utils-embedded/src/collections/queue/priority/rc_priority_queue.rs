use cellex_utils_core_rs::{
  collections::queue::{
    priority::{PriorityMessage, PriorityQueue, PRIORITY_LEVELS},
    traits::{QueueBase, QueueReader, QueueRw, QueueWriter},
    QueueSize,
  },
  v2::collections::queue::backend::QueueError,
};

use crate::collections::queue::ring::RcRingQueue;

#[cfg(test)]
mod tests;

/// `Rc`-based priority queue
///
/// This queue is available in `no_std` environments and controls processing order
/// based on message priority. It provides reference-counted shared ownership
/// using `Rc` and `RefCell`.
///
/// # Features
///
/// - **Priority-based**: Determines processing order based on message priority (0-7)
/// - **Multiple levels**: Supports 8 priority levels
/// - **Dynamic/static modes**: Choose between dynamic expansion or fixed capacity
/// - **no_std support**: Does not require the standard library
/// - **Cloneable**: Multiple handles can be created via `clone()`
///
/// # Priority
///
/// - Priority ranges from 0 (lowest) to 7 (highest) across 8 levels
/// - Default priority (0) is used when priority is not specified
/// - Higher priority messages are processed first
///
/// # Performance characteristics
///
/// - `offer`: O(1)
/// - `poll`: O(PRIORITY_LEVELS), typically close to O(1)
/// - Memory usage: O(capacity_per_level * PRIORITY_LEVELS)
///
/// # Examples
///
/// ```
/// use cellex_utils_core_rs::{PriorityMessage, QueueRw};
/// use cellex_utils_embedded_rs::prelude::RcPriorityQueue;
///
/// #[derive(Debug)]
/// struct Task {
///   id:       u32,
///   priority: i8,
/// }
///
/// impl PriorityMessage for Task {
///   fn get_priority(&self) -> Option<i8> {
///     Some(self.priority)
///   }
/// }
///
/// let queue = RcPriorityQueue::new(10);
/// queue.offer(Task { id: 1, priority: 0 }).unwrap();
/// queue.offer(Task { id: 2, priority: 5 }).unwrap();
///
/// // Higher priority task is retrieved first
/// let task = queue.poll().unwrap().unwrap();
/// assert_eq!(task.id, 2);
/// ```
#[derive(Debug, Clone)]
pub struct RcPriorityQueue<E> {
  inner: PriorityQueue<RcRingQueue<E>, E>,
}

impl<E> RcPriorityQueue<E> {
  /// Creates a new priority queue with the specified capacity per priority level
  ///
  /// # Arguments
  ///
  /// * `capacity_per_level` - Maximum number of elements that can be stored in each priority level
  ///   queue
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::prelude::RcPriorityQueue;
  ///
  /// // Can store up to 10 elements per priority level
  /// let queue: RcPriorityQueue<u32> = RcPriorityQueue::new(10);
  /// ```
  #[must_use]
  pub fn new(capacity_per_level: usize) -> Self {
    let levels = (0..PRIORITY_LEVELS).map(|_| RcRingQueue::new(capacity_per_level)).collect();
    Self { inner: PriorityQueue::new(levels) }
  }

  /// Sets the dynamic expansion mode of the queue
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, automatically expands when capacity is insufficient. If `false`,
  ///   capacity limits are strictly enforced.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::prelude::RcPriorityQueue;
  ///
  /// let queue: RcPriorityQueue<i32> = RcPriorityQueue::new(5);
  /// queue.set_dynamic(false); // Fixed capacity mode
  /// ```
  pub fn set_dynamic(&self, dynamic: bool) {
    for queue in self.inner.levels() {
      queue.set_dynamic(dynamic);
    }
  }

  /// Sets the dynamic expansion mode and returns self (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, automatically expands when capacity is insufficient
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::prelude::RcPriorityQueue;
  ///
  /// let queue: RcPriorityQueue<i32> = RcPriorityQueue::new(5).with_dynamic(false);
  /// ```
  #[must_use]
  pub fn with_dynamic(self, dynamic: bool) -> Self {
    self.set_dynamic(dynamic);
    self
  }

  /// Returns an immutable reference to the internal priority level queues
  ///
  /// # Returns
  ///
  /// Reference to an array of 8 priority level queues
  #[must_use]
  pub fn levels(&self) -> &[RcRingQueue<E>] {
    self.inner.levels()
  }

  /// Returns a mutable reference to the internal priority level queues
  ///
  /// # Returns
  ///
  /// Mutable reference to an array of 8 priority level queues
  pub fn levels_mut(&mut self) -> &mut [RcRingQueue<E>] {
    self.inner.levels_mut()
  }

  /// Returns an immutable reference to the internal `PriorityQueue`
  ///
  /// # Returns
  ///
  /// Reference to the internal `PriorityQueue` instance
  #[must_use]
  pub const fn inner(&self) -> &PriorityQueue<RcRingQueue<E>, E> {
    &self.inner
  }
}

impl<E> QueueBase<E> for RcPriorityQueue<E>
where
  E: PriorityMessage,
{
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E> QueueWriter<E> for RcPriorityQueue<E>
where
  E: PriorityMessage,
{
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E> QueueReader<E> for RcPriorityQueue<E>
where
  E: PriorityMessage,
{
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E> QueueRw<E> for RcPriorityQueue<E>
where
  E: PriorityMessage,
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
