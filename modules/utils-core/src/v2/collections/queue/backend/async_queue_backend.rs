use alloc::boxed::Box;

use async_trait::async_trait;

use super::{OfferOutcome, QueueError};

/// Async-compatible backend trait for queue operations.
#[async_trait(?Send)]
pub trait AsyncQueueBackend<T> {
  /// Adds an element to the queue according to the configured overflow policy.
  async fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError>;

  /// Removes and returns the next element from the queue.
  async fn poll(&mut self) -> Result<T, QueueError>;

  /// Transitions the backend into the closed state.
  async fn close(&mut self) -> Result<(), QueueError>;

  /// Returns the number of elements currently stored.
  fn len(&self) -> usize;

  /// Returns the maximum number of elements that can be stored without growing.
  fn capacity(&self) -> usize;

  /// Indicates whether the queue is empty.
  fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Indicates whether the queue is full.
  fn is_full(&self) -> bool {
    self.len() == self.capacity()
  }
}
