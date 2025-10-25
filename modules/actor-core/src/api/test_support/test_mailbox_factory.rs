#[cfg(feature = "queue-v2")]
use cellex_utils_core_rs::v2::collections::queue::backend::OverflowPolicy;
use cellex_utils_core_rs::Element;
#[cfg(not(feature = "queue-v2"))]
use cellex_utils_core_rs::MpscQueue;

#[cfg(not(feature = "queue-v2"))]
use crate::api::test_support::shared_backend_handle::SharedBackendHandle;
use crate::api::{
  mailbox::{
    queue_mailbox::{LegacyQueueDriver, QueueMailbox},
    MailboxFactory, MailboxOptions, MailboxPair, QueueMailboxProducer, ThreadSafe,
  },
  test_support::{common::TestQueue, test_signal::TestSignal},
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
    = LegacyQueueDriver<TestQueue<M>>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    #[cfg(feature = "queue-v2")]
    let queue = match capacity {
      | Some(0) | None => LegacyQueueDriver::new(TestQueue::unbounded()),
      | Some(limit) => LegacyQueueDriver::new(TestQueue::bounded(limit, OverflowPolicy::Block)),
    };
    #[cfg(not(feature = "queue-v2"))]
    let queue = LegacyQueueDriver::new(MpscQueue::new(SharedBackendHandle::new(capacity)));
    let signal = TestSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
