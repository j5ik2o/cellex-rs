use cellex_utils_core_rs::collections::queue::QueueSize;

/// Default number of reserved slots for control/system messages.
pub const DEFAULT_SYSTEM_RESERVATION: usize = 4;

/// Runtime-agnostic construction options for
/// [`QueueMailbox`](crate::api::mailbox::queue_mailbox::QueueMailbox).
///
/// Holds the capacity settings for mailboxes.
/// Different capacities can be set for regular messages and priority messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailboxOptions {
  /// Capacity for regular message queue
  pub capacity:          QueueSize,
  /// Capacity for priority message queue
  pub priority_capacity: QueueSize,
}

impl MailboxOptions {
  /// Creates mailbox options with the specified capacity.
  ///
  /// The priority message queue becomes unlimited.
  ///
  /// # Arguments
  /// - `capacity`: Capacity for regular message queue
  #[must_use]
  pub const fn with_capacity(capacity: usize) -> Self {
    Self { capacity: QueueSize::limited(capacity), priority_capacity: QueueSize::limitless() }
  }

  /// Creates mailbox options with both regular and priority capacities specified.
  ///
  /// # Arguments
  /// - `capacity`: Capacity for regular message queue
  /// - `priority_capacity`: Capacity for priority message queue
  #[must_use]
  pub const fn with_capacities(capacity: QueueSize, priority_capacity: QueueSize) -> Self {
    Self { capacity, priority_capacity }
  }

  /// Sets the capacity for the priority message queue.
  ///
  /// # Arguments
  /// - `priority_capacity`: Capacity for priority message queue
  #[must_use]
  pub const fn with_priority_capacity(mut self, priority_capacity: QueueSize) -> Self {
    self.priority_capacity = priority_capacity;
    self
  }

  /// Creates mailbox options with unlimited capacity.
  #[must_use]
  pub const fn unbounded() -> Self {
    Self { capacity: QueueSize::limitless(), priority_capacity: QueueSize::limitless() }
  }

  /// Returns the configured capacity limit for regular messages.
  ///
  /// When the mailbox is unbounded, this returns `None`. Otherwise it
  /// contains the finite capacity.
  #[must_use]
  pub const fn capacity_limit(&self) -> Option<usize> {
    match self.capacity {
      | QueueSize::Limitless => None,
      | QueueSize::Limited(value) => Some(value),
    }
  }

  /// Returns the configured capacity limit for priority messages.
  ///
  /// When the priority queue is unbounded, this returns `None`.
  #[must_use]
  pub const fn priority_capacity_limit(&self) -> Option<usize> {
    match self.priority_capacity {
      | QueueSize::Limitless => None,
      | QueueSize::Limited(value) => Some(value),
    }
  }
}

impl Default for MailboxOptions {
  fn default() -> Self {
    Self {
      capacity:          QueueSize::limitless(),
      priority_capacity: QueueSize::limited(DEFAULT_SYSTEM_RESERVATION),
    }
  }
}
