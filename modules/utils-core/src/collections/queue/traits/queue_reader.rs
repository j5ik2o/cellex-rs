use crate::{collections::queue::traits::queue_base::QueueBase, v2::collections::queue::backend::QueueError};

/// Trait providing read operations from the queue for mutable references.
pub trait QueueReader<E>: QueueBase<E> {
  /// Removes an element from the queue (mutable reference version).
  fn poll_mut(&mut self) -> Result<Option<E>, QueueError<E>>;

  /// Performs queue cleanup processing (mutable reference version).
  fn clean_up_mut(&mut self);
}
