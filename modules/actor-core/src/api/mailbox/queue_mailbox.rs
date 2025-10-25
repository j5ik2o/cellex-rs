mod base;
mod core;
mod driver;
mod legacy_queue_driver;
mod poll_outcome;
mod recv;
#[cfg(feature = "queue-v2")]
mod sync_queue_driver;

pub use core::MailboxQueueCore;

pub use base::QueueMailbox;
#[cfg(feature = "queue-v2")]
use cellex_utils_core_rs::{Element, QueueSize};
pub use driver::MailboxQueueDriver;
pub use legacy_queue_driver::LegacyQueueDriver;
pub use poll_outcome::QueuePollOutcome;
pub use recv::QueueMailboxRecv;
#[cfg(feature = "queue-v2")]
pub use sync_queue_driver::SyncQueueDriver;

#[cfg(all(test, feature = "queue-v2"))]
mod tests;

#[cfg(feature = "queue-v2")]
use crate::api::mailbox::MailboxOverflowPolicy;

/// Default queue driver type that `QueueMailbox` uses when constructing drivers internally.
#[cfg(feature = "queue-v2")]
pub type DefaultQueueDriver<M> = SyncQueueDriver<M>;

/// Configuration for constructing a queue driver.
#[cfg(feature = "queue-v2")]
#[derive(Clone, Copy, Debug)]
pub struct QueueDriverConfig {
  /// Queue capacity that governs how the driver allocates storage.
  pub capacity:        QueueSize,
  /// Overflow handling policy applied when the queue reaches capacity.
  pub overflow_policy: MailboxOverflowPolicy,
}

#[cfg(feature = "queue-v2")]
impl QueueDriverConfig {
  #[must_use]
  /// Creates a new configuration using the supplied capacity and overflow policy.
  pub const fn new(capacity: QueueSize, overflow_policy: MailboxOverflowPolicy) -> Self {
    Self { capacity, overflow_policy }
  }
}

#[cfg(feature = "queue-v2")]
impl Default for QueueDriverConfig {
  /// Provides the default configuration matching unlimited capacity with growth.
  fn default() -> Self {
    Self { capacity: QueueSize::limitless(), overflow_policy: MailboxOverflowPolicy::Grow }
  }
}

/// Builds a queue driver according to the supplied configuration.
#[cfg(feature = "queue-v2")]
pub fn build_queue_driver<M>(config: QueueDriverConfig) -> DefaultQueueDriver<M>
where
  M: Element, {
  use cellex_utils_core_rs::v2::collections::queue::backend::OverflowPolicy;

  let policy = match config.overflow_policy {
    | MailboxOverflowPolicy::DropNewest => OverflowPolicy::DropNewest,
    | MailboxOverflowPolicy::DropOldest => OverflowPolicy::DropOldest,
    | MailboxOverflowPolicy::Grow => OverflowPolicy::Grow,
    | MailboxOverflowPolicy::Block => OverflowPolicy::Block,
  };

  match config.capacity {
    | QueueSize::Limitless => SyncQueueDriver::unbounded(),
    | QueueSize::Limited(limit) => SyncQueueDriver::bounded(limit.max(1), policy),
  }
}
