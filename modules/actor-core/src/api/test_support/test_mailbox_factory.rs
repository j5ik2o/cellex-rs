use core::any::TypeId;

use cellex_utils_core_rs::collections::{queue::QueueSize, Element};

use crate::{
  api::{
    mailbox::{
      messages::SystemMessage,
      queue_mailbox::{
        build_user_mailbox_queue, MailboxQueueConfig, QueueMailbox, SystemMailboxQueue, UserMailboxQueue,
      },
      MailboxOverflowPolicy, QueueMailboxProducer, ThreadSafe,
    },
    test_support::test_signal::TestSignal,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory, MailboxOptions, MailboxPair},
    messaging::AnyMessage,
  },
};

#[derive(Clone, Debug, Default)]
/// Minimal [`MailboxFactory`] implementation used by unit tests to build queue-backed mailboxes.
pub struct TestMailboxFactory {
  capacity: Option<usize>,
}

impl TestMailboxFactory {
  /// Creates a runtime with an optional global capacity shared by all queues.
  #[must_use]
  pub const fn new(capacity: Option<usize>) -> Self {
    Self { capacity }
  }

  /// Creates a runtime that enforces the same capacity for every mailbox queue.
  #[must_use]
  pub const fn with_capacity_per_queue(capacity: usize) -> Self {
    Self::new(Some(capacity))
  }

  /// Creates an unbounded runtime that lets queue options decide the capacity.
  #[must_use]
  pub fn unbounded() -> Self {
    Self::default()
  }

  const fn resolve_capacity(&self, options: MailboxOptions) -> Option<usize> {
    match options.capacity_limit() {
      | Some(limit) => Some(limit),
      | None => self.capacity,
    }
  }
}

impl MailboxFactory for TestMailboxFactory {
  type Concurrency = ThreadSafe;
  type Mailbox<M>
    = QueueMailbox<Self::Queue<M>, UserMailboxQueue<M>, Self::Signal>
  where
    M: Element;
  type Producer<M>
    = QueueMailboxProducer<Self::Queue<M>, UserMailboxQueue<M>, Self::Signal>
  where
    M: Element;
  type Queue<M>
    = SystemMailboxQueue<M>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    let (system_queue, user_queue) = {
      let capacity_size = match capacity {
        | Some(0) | None => QueueSize::limitless(),
        | Some(limit) => QueueSize::limited(limit),
      };
      let config = MailboxQueueConfig::new(capacity_size, MailboxOverflowPolicy::Block);
      let user_queue = build_user_mailbox_queue::<M>(config);
      let system_capacity = if TypeId::of::<M>() == TypeId::of::<PriorityEnvelope<AnyMessage>>()
        || TypeId::of::<M>() == TypeId::of::<PriorityEnvelope<SystemMessage>>()
      {
        let capacity = options.priority_capacity_limit();
        #[cfg(debug_assertions)]
        {
          debug_assert!(capacity.is_some(), "priority capacity not configured for priority mailbox");
        }
        capacity
      } else {
        None
      };
      (SystemMailboxQueue::new(system_capacity), user_queue)
    };
    let signal = TestSignal::default();
    let mailbox = QueueMailbox::with_system_queue(system_queue, user_queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
