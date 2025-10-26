use cellex_utils_core_rs::collections::queue::QueueError;

/// Result of polling the underlying queue.
pub enum QueuePollOutcome<M> {
  /// A message was dequeued successfully.
  Message(M),
  /// No message is currently available.
  Empty,
  /// The queue cannot deliver data yet and the caller should retry later.
  Pending,
  /// The queue has been disconnected permanently.
  Disconnected,
  /// The queue has been closed and returns the preserved message.
  Closed(M),
  /// An exceptional error was reported by the queue implementation.
  Err(QueueError<M>),
}

impl<M> QueuePollOutcome<M> {
  /// Creates an outcome from a low-level queue result.
  pub fn from_result(result: Result<Option<M>, QueueError<M>>) -> Self {
    match result {
      | Ok(Some(message)) => Self::Message(message),
      | Ok(None) | Err(QueueError::Empty) => Self::Empty,
      | Err(QueueError::WouldBlock) => Self::Pending,
      | Err(QueueError::Disconnected) => Self::Disconnected,
      | Err(QueueError::Closed(message)) => Self::Closed(message),
      | Err(other) => Self::Err(other),
    }
  }
}
