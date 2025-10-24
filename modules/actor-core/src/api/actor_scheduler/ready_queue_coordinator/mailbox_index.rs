//! MailboxIndex - Mailbox identification with generational safety

/// Mailbox index with slot and generation for safe reuse
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MailboxIndex {
  /// Slot number in the registry
  pub slot:       u32,
  /// Generation number to prevent use-after-free
  pub generation: u32,
}

impl MailboxIndex {
  /// Create a new MailboxIndex
  pub fn new(slot: u32, generation: u32) -> Self {
    Self { slot, generation }
  }
}
