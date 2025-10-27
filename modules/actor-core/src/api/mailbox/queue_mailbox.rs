mod backend;
mod base;
mod core;
mod poll_outcome;
mod queue;
mod recv;
mod system_mailbox_lane;
mod system_mailbox_queue;
mod user_mailbox_queue;

pub use core::QueueMailboxCore;

pub use backend::MailboxQueueBackend;
pub use base::QueueMailbox;
use cellex_utils_core_rs::collections::{queue::QueueSize, Element};
pub use poll_outcome::QueuePollOutcome;
pub(crate) use queue::MailboxQueue;
pub use recv::QueueMailboxRecv;
pub use system_mailbox_lane::SystemMailboxLane;
pub use system_mailbox_queue::SystemMailboxQueue;
pub use user_mailbox_queue::UserMailboxQueue;

#[cfg(test)]
mod tests;

use crate::{
  api::mailbox::{queue_mailbox_producer::QueueMailboxProducer, MailboxOverflowPolicy},
  shared::mailbox::MailboxSignal,
};

/// Mailbox alias for a queue that omits the system lane and relies solely on [`UserMailboxQueue`].
pub type UserOnlyMailbox<M, S> = QueueMailbox<(), UserMailboxQueue<M>, S>;

/// Producer alias associated with [`UserOnlyMailbox`].
pub type UserOnlyMailboxProducer<M, S> = QueueMailboxProducer<(), UserMailboxQueue<M>, S>;

/// Receive future alias associated with [`UserOnlyMailbox`].
pub type UserOnlyMailboxRecv<'a, S, M> = QueueMailboxRecv<'a, (), UserMailboxQueue<M>, S, M>;

/// Mailbox alias that composes a [`SystemMailboxQueue`] reservation lane with a
/// [`UserMailboxQueue`].
pub type DefaultMailbox<M, S> = QueueMailbox<SystemMailboxQueue<M>, UserMailboxQueue<M>, S>;

/// Producer alias associated with [`DefaultMailbox`].
pub type DefaultMailboxProducer<M, S> = QueueMailboxProducer<SystemMailboxQueue<M>, UserMailboxQueue<M>, S>;

/// Receive future alias associated with [`DefaultMailbox`].
pub type DefaultMailboxRecv<'a, S, M> = QueueMailboxRecv<'a, SystemMailboxQueue<M>, UserMailboxQueue<M>, S, M>;

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

/// Builds a mailbox/producer pair that wires together both system and user queues using the
/// supplied signal.
pub fn build_default_mailbox_pair<M, S>(
  signal: S,
  config: MailboxQueueConfig,
) -> (DefaultMailbox<M, S>, DefaultMailboxProducer<M, S>)
where
  M: Element,
  S: MailboxSignal + Clone, {
  let user_queue = build_user_mailbox_queue::<M>(config);
  let system_queue = SystemMailboxQueue::new(Some(crate::shared::mailbox::DEFAULT_SYSTEM_RESERVATION));
  let mailbox = QueueMailbox::with_system_queue(system_queue, user_queue, signal);
  let producer = mailbox.producer();
  (mailbox, producer)
}

impl Default for MailboxQueueConfig {
  /// Provides the default configuration matching unlimited capacity with growth.
  fn default() -> Self {
    Self { capacity: QueueSize::limitless(), overflow_policy: MailboxOverflowPolicy::Grow }
  }
}

/// Builds a user mailbox queue according to the supplied configuration.
pub fn build_user_mailbox_queue<M>(config: MailboxQueueConfig) -> UserMailboxQueue<M>
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
    | QueueSize::Limitless => UserMailboxQueue::unbounded(),
    | QueueSize::Limited(limit) => UserMailboxQueue::bounded(limit.max(1), policy),
  }
}
