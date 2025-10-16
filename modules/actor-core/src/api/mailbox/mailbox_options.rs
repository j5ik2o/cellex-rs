use cellex_utils_core_rs::QueueSize;

/// Runtime-agnostic construction options for [`QueueMailbox`].
///
/// Holds the capacity settings for mailboxes.
/// Different capacities can be set for regular messages and priority messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailboxOptions {
  /// Capacity for regular message queue
  pub capacity: QueueSize,
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
  pub const fn with_capacity(capacity: usize) -> Self {
    Self {
      capacity: QueueSize::limited(capacity),
      priority_capacity: QueueSize::limitless(),
    }
  }

  /// Creates mailbox options with both regular and priority capacities specified.
  ///
  /// # Arguments
  /// - `capacity`: Capacity for regular message queue
  /// - `priority_capacity`: Capacity for priority message queue
  pub const fn with_capacities(capacity: QueueSize, priority_capacity: QueueSize) -> Self {
    Self {
      capacity,
      priority_capacity,
    }
  }

  /// Sets the capacity for the priority message queue.
  ///
  /// # Arguments
  /// - `priority_capacity`: Capacity for priority message queue
  pub const fn with_priority_capacity(mut self, priority_capacity: QueueSize) -> Self {
    self.priority_capacity = priority_capacity;
    self
  }

  /// Creates mailbox options with unlimited capacity.
  pub const fn unbounded() -> Self {
    Self {
      capacity: QueueSize::limitless(),
      priority_capacity: QueueSize::limitless(),
    }
  }
}

impl Default for MailboxOptions {
  fn default() -> Self {
    Self::unbounded()
  }
}
