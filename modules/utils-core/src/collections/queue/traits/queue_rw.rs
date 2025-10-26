use crate::{collections::queue::traits::queue_base::QueueBase, v2::collections::queue::backend::QueueError};

/// Trait providing read/write operations for the queue using shared references.
pub trait QueueRw<E>: QueueBase<E> {
  /// Adds an element to the queue (shared reference version).
  fn offer(&self, element: E) -> Result<(), QueueError<E>>;

  /// Removes an element from the queue (shared reference version).
  fn poll(&self) -> Result<Option<E>, QueueError<E>>;

  /// Performs queue cleanup processing (shared reference version).
  fn clean_up(&self);
}
