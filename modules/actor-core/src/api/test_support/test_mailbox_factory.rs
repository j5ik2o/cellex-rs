use cellex_utils_core_rs::collections::{queue::QueueSize, Element};

use crate::{
  api::{
    mailbox::{
      queue_mailbox::{build_mailbox_queue, MailboxQueueConfig, QueueMailbox, SyncMailboxQueue},
      MailboxOverflowPolicy, QueueMailboxProducer, ThreadSafe,
    },
    test_support::test_signal::TestSignal,
  },
  shared::mailbox::{MailboxFactory, MailboxOptions, MailboxPair},
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
    = QueueMailbox<Self::Queue<M>, Self::Signal>
  where
    M: Element;
  type Producer<M>
    = QueueMailboxProducer<Self::Queue<M>, Self::Signal>
  where
    M: Element;
  type Queue<M>
    = SyncMailboxQueue<M>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    let queue = {
      let capacity_size = match capacity {
        | Some(0) | None => QueueSize::limitless(),
        | Some(limit) => QueueSize::limited(limit),
      };
      let config = MailboxQueueConfig::new(capacity_size, MailboxOverflowPolicy::Block);
      build_mailbox_queue::<M>(config)
    };
    let signal = TestSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
