use core::future::Future;

/// Notification primitive used by `QueueMailbox` to park awaiting receivers until
/// new messages are available.
///
/// Synchronization primitive used for notifying message arrivals.
/// Provides a mechanism for receivers to wait for messages and senders to notify arrivals.
pub trait MailboxSignal: Clone {
  /// Future type for waiting
  type WaitFuture<'a>: Future<Output = ()> + 'a
  where
    Self: 'a;

  /// Notifies waiting receivers that a message has arrived.
  fn notify(&self);

  /// Waits for a message arrival.
  ///
  /// # Returns
  /// Future that waits for notification
  fn wait(&self) -> Self::WaitFuture<'_>;
}
