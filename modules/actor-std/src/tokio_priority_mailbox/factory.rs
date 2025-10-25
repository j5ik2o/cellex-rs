use cellex_actor_core_rs::api::mailbox::{
  queue_mailbox::{LegacyQueueDriver, QueueMailbox},
  MailboxOptions,
};
use cellex_utils_std_rs::{Element, QueueSize, DEFAULT_CAPACITY, PRIORITY_LEVELS};

use super::{
  mailbox::TokioPriorityMailbox, queues::TokioPriorityQueues, sender::TokioPriorityMailboxSender, NotifySignal,
};

/// Factory that creates priority mailboxes
///
/// Configures the capacity of control and regular queues and the number of priority levels,
/// and creates mailbox instances.
#[derive(Clone, Debug)]
pub struct TokioPriorityMailboxFactory {
  control_capacity_per_level: usize,
  regular_capacity:           usize,
  levels:                     usize,
}

impl Default for TokioPriorityMailboxFactory {
  fn default() -> Self {
    Self {
      control_capacity_per_level: DEFAULT_CAPACITY,
      regular_capacity:           DEFAULT_CAPACITY,
      levels:                     PRIORITY_LEVELS,
    }
  }
}

impl TokioPriorityMailboxFactory {
  /// Creates a new factory instance
  ///
  /// # Arguments
  ///
  /// * `control_capacity_per_level` - Capacity per priority level for the control queue
  ///
  /// # Returns
  ///
  /// A factory initialized with default regular queue capacity and default number of priority
  /// levels
  #[allow(clippy::missing_const_for_fn)]
  #[must_use]
  pub fn new(control_capacity_per_level: usize) -> Self {
    Self { control_capacity_per_level, regular_capacity: DEFAULT_CAPACITY, levels: PRIORITY_LEVELS }
  }

  /// Sets the number of priority levels (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `levels` - Number of priority levels to set (minimum value is 1)
  ///
  /// # Returns
  ///
  /// Factory instance with updated settings
  #[allow(clippy::missing_const_for_fn)]
  #[must_use]
  pub fn with_levels(mut self, levels: usize) -> Self {
    self.levels = levels.max(1);
    self
  }

  /// Sets the regular queue capacity (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `capacity` - Capacity of the regular message queue
  ///
  /// # Returns
  ///
  /// Factory instance with updated settings
  #[allow(clippy::missing_const_for_fn)]
  #[must_use]
  pub fn with_regular_capacity(mut self, capacity: usize) -> Self {
    self.regular_capacity = capacity;
    self
  }

  /// Creates a pair of mailbox and sender handle
  ///
  /// # Arguments
  ///
  /// * `options` - Mailbox capacity options
  ///
  /// # Returns
  ///
  /// `(TokioPriorityMailbox<M>, TokioPriorityMailboxSender<M>)` - Tuple of mailbox and sender
  /// handle
  #[must_use]
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (TokioPriorityMailbox<M>, TokioPriorityMailboxSender<M>)
  where
    M: Element, {
    let control_per_level = self.resolve_control_capacity(options.priority_capacity);
    let regular_capacity = self.resolve_regular_capacity(options.capacity);
    let queue = LegacyQueueDriver::new(TokioPriorityQueues::<M>::new(self.levels, control_per_level, regular_capacity));
    let signal = NotifySignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (TokioPriorityMailbox::from_inner(mailbox), TokioPriorityMailboxSender::from_inner(sender))
  }

  const fn resolve_control_capacity(&self, requested: QueueSize) -> usize {
    match requested {
      | QueueSize::Limitless => self.control_capacity_per_level,
      | QueueSize::Limited(value) => value,
    }
  }

  const fn resolve_regular_capacity(&self, requested: QueueSize) -> usize {
    match requested {
      | QueueSize::Limitless => self.regular_capacity,
      | QueueSize::Limited(value) => value,
    }
  }
}
