use core::marker::PhantomData;

use cellex_actor_core_rs::{api::mailbox::queue_mailbox::QueueMailbox, shared::mailbox::MailboxOptions};
use cellex_utils_core_rs::collections::{
  queue::{priority::PRIORITY_LEVELS, QueueSize},
  Element,
};

/// Default capacity for mailbox queues
const DEFAULT_CAPACITY: usize = 32;
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{
  mailbox::ArcPriorityMailbox, priority_mailbox_queue::PriorityMailboxQueue, sender::ArcPriorityMailboxSender,
};
use crate::arc_mailbox::ArcSignal;

/// Factory for constructing [`ArcPriorityMailbox`] instances.
#[derive(Debug)]
pub struct ArcPriorityMailboxFactory<RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  RM: RawMutex, {
  control_capacity_per_level: usize,
  regular_capacity:           usize,
  levels:                     usize,
  _marker:                    PhantomData<RM>,
}

impl<RM> Default for ArcPriorityMailboxFactory<RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self {
      control_capacity_per_level: DEFAULT_CAPACITY,
      regular_capacity:           DEFAULT_CAPACITY,
      levels:                     PRIORITY_LEVELS,
      _marker:                    PhantomData,
    }
  }
}

impl<RM> ArcPriorityMailboxFactory<RM>
where
  RM: RawMutex,
{
  /// Creates a new factory with the specified control capacity per priority level.
  pub const fn new(control_capacity_per_level: usize) -> Self {
    Self {
      control_capacity_per_level,
      regular_capacity: DEFAULT_CAPACITY,
      levels: PRIORITY_LEVELS,
      _marker: PhantomData,
    }
  }

  /// Updates the number of priority levels managed by the factory.
  pub fn with_levels(mut self, levels: usize) -> Self {
    self.levels = levels.max(1);
    self
  }

  /// Updates the capacity dedicated to regular (non-control) messages.
  pub fn with_regular_capacity(mut self, capacity: usize) -> Self {
    self.regular_capacity = capacity;
    self
  }

  /// Builds a mailbox using the provided options.
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (ArcPriorityMailbox<M, RM>, ArcPriorityMailboxSender<M, RM>)
  where
    M: Element, {
    let control_per_level = self.resolve_control_capacity(options.priority_capacity);
    let regular_capacity = self.resolve_regular_capacity(options.capacity);
    let queue = PriorityMailboxQueue::new(self.levels, control_per_level, regular_capacity);
    let signal = ArcSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (ArcPriorityMailbox { inner: mailbox }, ArcPriorityMailboxSender { inner: sender })
  }

  fn resolve_control_capacity(&self, requested: QueueSize) -> usize {
    match requested {
      | QueueSize::Limitless => self.control_capacity_per_level,
      | QueueSize::Limited(value) => value,
    }
  }

  fn resolve_regular_capacity(&self, requested: QueueSize) -> usize {
    match requested {
      | QueueSize::Limitless => self.regular_capacity,
      | QueueSize::Limited(value) => value,
    }
  }
}

impl<RM> Clone for ArcPriorityMailboxFactory<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self {
      control_capacity_per_level: self.control_capacity_per_level,
      regular_capacity:           self.regular_capacity,
      levels:                     self.levels,
      _marker:                    PhantomData,
    }
  }
}
