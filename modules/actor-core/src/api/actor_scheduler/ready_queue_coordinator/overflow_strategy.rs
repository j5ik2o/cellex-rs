//! OverflowStrategy - Mailbox capacity overflow handling

/// Overflow strategy for mailbox capacity limits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowStrategy {
  /// Drop the oldest message when full
  DropOldest,
  /// Drop the newest message when full
  DropNewest,
  /// Block the producer until space is available
  BlockProducer,
  /// Reject the message immediately
  Reject,
  /// Send to dead letter queue
  DeadLetter,
}
