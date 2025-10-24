use crate::v2::collections::queue::{storage::QueueStorage, OfferOutcome, OverflowPolicy, QueueError};

/// Backend trait responsible for queue operations on top of a storage implementation.
pub trait SyncQueueBackend<T> {
  /// Storage implementation backing the queue.
  type Storage: QueueStorage<T>;

  /// Constructs a new backend configured with the provided storage and overflow policy.
  fn new(storage: Self::Storage, policy: OverflowPolicy) -> Self;

  /// Adds an element to the queue according to the configured overflow policy.
  fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError>;

  /// Removes and returns the next element from the queue.
  fn poll(&mut self) -> Result<T, QueueError>;

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

  /// Closes the backend, preventing further offers while allowing in-flight polls to complete.
  fn close(&mut self) {}
}
