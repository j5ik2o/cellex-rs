#[cfg(test)]
mod tests;

use alloc::vec::Vec;

use cellex_utils_core_rs::{
  collections::queue::{
    priority::{PriorityMessage, PriorityQueue, PRIORITY_LEVELS},
    traits::{QueueBase, QueueReader, QueueRw, QueueWriter},
    QueueSize,
  },
  v2::collections::queue::backend::QueueError,
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex};

use crate::collections::queue::ring::ArcRingQueue;

/// Type alias for `ArcPriorityQueue` using `NoopRawMutex`
///
/// Suitable for single-threaded or interrupt-free contexts where no locking is required.
pub type ArcLocalPriorityQueue<E> = ArcPriorityQueue<E, NoopRawMutex>;

/// Type alias for `ArcPriorityQueue` using `CriticalSectionRawMutex`
///
/// Provides interrupt-safe critical section protection for multi-threaded embedded contexts.
pub type ArcCsPriorityQueue<E> = ArcPriorityQueue<E, CriticalSectionRawMutex>;

/// `Arc`-based priority queue with configurable mutex backend
///
/// This queue provides priority-based message ordering with 8 priority levels (0-7),
/// using `Arc` for thread-safe reference counting. The mutex backend is configurable
/// via the `RM` type parameter.
///
/// # Type Parameters
///
/// * `E` - Element type, must implement `PriorityMessage`
/// * `RM` - Raw mutex type (defaults to `NoopRawMutex`)
#[derive(Debug)]
pub struct ArcPriorityQueue<E, RM = NoopRawMutex>
where
  RM: RawMutex, {
  inner: PriorityQueue<ArcRingQueue<E, RM>, E>,
}

impl<E, RM> Clone for ArcPriorityQueue<E, RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    let levels: Vec<_> = self.inner.levels().iter().cloned().collect();
    Self { inner: PriorityQueue::new(levels) }
  }
}

impl<E, RM> ArcPriorityQueue<E, RM>
where
  RM: RawMutex,
{
  /// Creates a new priority queue with the specified capacity per priority level
  ///
  /// # Arguments
  ///
  /// * `capacity_per_level` - Maximum number of elements per priority level
  pub fn new(capacity_per_level: usize) -> Self {
    let levels = (0..PRIORITY_LEVELS).map(|_| ArcRingQueue::new(capacity_per_level)).collect();
    Self { inner: PriorityQueue::new(levels) }
  }

  /// Sets the dynamic expansion mode for all priority levels
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, queues expand dynamically; if `false`, capacity is fixed
  pub fn set_dynamic(&self, dynamic: bool) {
    for queue in self.inner.levels() {
      queue.set_dynamic(dynamic);
    }
  }

  /// Sets the dynamic expansion mode and returns self (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `dynamic` - If `true`, queues expand dynamically
  pub fn with_dynamic(self, dynamic: bool) -> Self {
    self.set_dynamic(dynamic);
    self
  }

  /// Returns immutable references to the internal priority level queues
  ///
  /// # Returns
  ///
  /// Slice of 8 priority level queues
  pub fn levels(&self) -> &[ArcRingQueue<E, RM>] {
    self.inner.levels()
  }

  /// Returns mutable references to the internal priority level queues
  ///
  /// # Returns
  ///
  /// Mutable slice of 8 priority level queues
  pub fn levels_mut(&mut self) -> &mut [ArcRingQueue<E, RM>] {
    self.inner.levels_mut()
  }

  /// Returns a reference to the internal `PriorityQueue`
  ///
  /// # Returns
  ///
  /// Reference to the underlying priority queue implementation
  pub fn inner(&self) -> &PriorityQueue<ArcRingQueue<E, RM>, E> {
    &self.inner
  }
}

impl<E, RM> QueueBase<E> for ArcPriorityQueue<E, RM>
where
  E: PriorityMessage,
  RM: RawMutex,
{
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<E, RM> QueueWriter<E> for ArcPriorityQueue<E, RM>
where
  E: PriorityMessage,
  RM: RawMutex,
{
  fn offer_mut(&mut self, element: E) -> Result<(), QueueError<E>> {
    self.inner.offer_mut(element)
  }
}

impl<E, RM> QueueReader<E> for ArcPriorityQueue<E, RM>
where
  E: PriorityMessage,
  RM: RawMutex,
{
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>> {
    self.inner.poll_mut()
  }

  fn clean_up_mut(&mut self) {
    self.inner.clean_up_mut();
  }
}

impl<E, RM> QueueRw<E> for ArcPriorityQueue<E, RM>
where
  E: PriorityMessage,
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
