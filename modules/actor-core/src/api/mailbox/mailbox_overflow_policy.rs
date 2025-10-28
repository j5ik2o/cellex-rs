//! Overflow policy abstractions shared by mailbox errors.

use cellex_utils_core_rs::collections::queue::backend::OverflowPolicy;

/// Policies describing how a mailbox reacts when it reaches capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxOverflowPolicy {
  /// The newest element is dropped while enqueueing.
  DropNewest,
  /// The oldest element is removed to make room for the new one.
  DropOldest,
  /// The queue grows dynamically to accommodate more elements.
  Grow,
  /// The producer blocks or retries until capacity becomes available.
  Block,
}

impl From<OverflowPolicy> for MailboxOverflowPolicy {
  fn from(policy: OverflowPolicy) -> Self {
    match policy {
      | OverflowPolicy::DropNewest => Self::DropNewest,
      | OverflowPolicy::DropOldest => Self::DropOldest,
      | OverflowPolicy::Grow => Self::Grow,
      | OverflowPolicy::Block => Self::Block,
    }
  }
}
