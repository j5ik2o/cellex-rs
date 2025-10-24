//! MailboxOptions - Mailbox configuration

use core::num::NonZeroUsize;

use super::OverflowStrategy;

/// Mailbox configuration options
#[derive(Debug, Clone)]
pub struct MailboxOptions {
  /// Maximum capacity (must be non-zero)
  pub capacity:           NonZeroUsize,
  /// Strategy when capacity is exceeded
  pub overflow:           OverflowStrategy,
  /// Reserved slots for system messages
  pub reserve_for_system: usize,
}

impl Default for MailboxOptions {
  fn default() -> Self {
    Self {
      capacity:           NonZeroUsize::new(1000).unwrap(),
      overflow:           OverflowStrategy::DropOldest,
      reserve_for_system: 10,
    }
  }
}
