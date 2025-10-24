use crate::v2::sync::SharedError;

/// Errors that may arise while operating on a queue backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueError {
  /// The queue cannot accept new elements.
  Full,
  /// The queue has no elements to consume.
  Empty,
  /// The queue has been closed and will not accept further operations.
  Closed,
  /// The underlying shared state is no longer accessible.
  Disconnected,
  /// The operation would block and cannot proceed in the current context.
  WouldBlock,
  /// Allocator-related failure occurred while growing the storage.
  AllocError,
}

impl From<SharedError> for QueueError {
  fn from(err: SharedError) -> Self {
    match err {
      | SharedError::Poisoned => QueueError::Disconnected,
      | SharedError::BorrowConflict => QueueError::WouldBlock,
      | SharedError::InterruptContext => QueueError::WouldBlock,
    }
  }
}
