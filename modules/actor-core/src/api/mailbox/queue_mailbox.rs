mod backend;
mod base;
mod core;
mod poll_outcome;
mod queue;
mod recv;
mod sync_mailbox_queue;

pub use core::QueueMailboxCore;

pub use backend::MailboxQueueBackend;
pub use base::QueueMailbox;
use cellex_utils_core_rs::collections::{queue::QueueSize, Element};
pub use poll_outcome::QueuePollOutcome;
pub(crate) use queue::MailboxQueue;
pub use recv::QueueMailboxRecv;
pub use sync_mailbox_queue::SyncMailboxQueue;

#[cfg(test)]
mod tests;

use crate::{
  api::mailbox::{queue_mailbox_producer::QueueMailboxProducer, MailboxOverflowPolicy},
  shared::mailbox::MailboxSignal,
};

/// Default queue type that `QueueMailbox` uses when constructing mailboxes internally.
pub type DefaultMailboxQueue<M> = SyncMailboxQueue<M>;

/// Convenience alias for the standard mailbox backed by [`SyncMailboxQueue`].
pub type SyncMailbox<M, S> = QueueMailbox<SyncMailboxQueue<M>, S>;

/// Producer alias associated with [`SyncMailbox`].
pub type SyncMailboxProducer<M, S> = QueueMailboxProducer<SyncMailboxQueue<M>, S>;

/// Receive future alias associated with [`SyncMailbox`].
pub type SyncMailboxRecv<'a, S, M> = QueueMailboxRecv<'a, SyncMailboxQueue<M>, S, M>;

/// Configuration for constructing a mailbox queue.
#[derive(Clone, Copy, Debug)]
pub struct MailboxQueueConfig {
  /// Queue capacity that governs how the driver allocates storage.
  pub capacity:        QueueSize,
  /// Overflow handling policy applied when the queue reaches capacity.
  pub overflow_policy: MailboxOverflowPolicy,
}

impl MailboxQueueConfig {
  #[must_use]
  /// Creates a new configuration using the supplied capacity and overflow policy.
  pub const fn new(capacity: QueueSize, overflow_policy: MailboxOverflowPolicy) -> Self {
    Self { capacity, overflow_policy }
  }
}

/// Builds a mailbox/producer pair backed by [`SyncMailboxQueue`] using the supplied signal and
/// configuration.
pub fn build_sync_mailbox_pair<M, S>(
  signal: S,
  config: MailboxQueueConfig,
) -> (SyncMailbox<M, S>, SyncMailboxProducer<M, S>)
where
  M: Element,
  S: MailboxSignal + Clone, {
  let queue = build_mailbox_queue::<M>(config);
  let mailbox = QueueMailbox::new(queue, signal);
  let producer = mailbox.producer();
  (mailbox, producer)
}

impl Default for MailboxQueueConfig {
  /// Provides the default configuration matching unlimited capacity with growth.
  fn default() -> Self {
    Self { capacity: QueueSize::limitless(), overflow_policy: MailboxOverflowPolicy::Grow }
  }
}

/// Builds a mailbox queue according to the supplied configuration.
pub fn build_mailbox_queue<M>(config: MailboxQueueConfig) -> DefaultMailboxQueue<M>
where
  M: Element, {
  use cellex_utils_core_rs::collections::queue::backend::OverflowPolicy;

  let policy = match config.overflow_policy {
    | MailboxOverflowPolicy::DropNewest => OverflowPolicy::DropNewest,
    | MailboxOverflowPolicy::DropOldest => OverflowPolicy::DropOldest,
    | MailboxOverflowPolicy::Grow => OverflowPolicy::Grow,
    | MailboxOverflowPolicy::Block => OverflowPolicy::Block,
  };

  match config.capacity {
    | QueueSize::Limitless => SyncMailboxQueue::unbounded(),
    | QueueSize::Limited(limit) => SyncMailboxQueue::bounded(limit.max(1), policy),
  }
}
