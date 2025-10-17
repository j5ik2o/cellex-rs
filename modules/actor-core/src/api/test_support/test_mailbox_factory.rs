use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::MailboxOptions;
use crate::api::mailbox::MailboxPair;
use crate::api::mailbox::QueueMailbox;
use crate::api::mailbox::QueueMailboxProducer;
use crate::api::mailbox::ThreadSafe;
use crate::api::test_support::common::TestQueue;
use crate::api::test_support::shared_backend_handle::SharedBackendHandle;
use crate::api::test_support::test_signal::TestSignal;
use cellex_utils_core_rs::{Element, MpscQueue, QueueSize};

#[derive(Clone, Debug, Default)]
/// Minimal use cellex_actor_core_rs::api::mailbox::MailboxRuntime; used by unit tests to build queue-backed mailboxes.
pub struct TestMailboxFactory {
  capacity: Option<usize>,
}

impl TestMailboxFactory {
  /// Creates a runtime with an optional global capacity shared by all queues.
  pub const fn new(capacity: Option<usize>) -> Self {
    Self { capacity }
  }

  /// Creates a runtime that enforces the same capacity for every mailbox queue.
  pub const fn with_capacity_per_queue(capacity: usize) -> Self {
    Self::new(Some(capacity))
  }

  /// Creates an unbounded runtime that lets queue options decide the capacity.
  pub fn unbounded() -> Self {
    Self::default()
  }

  const fn resolve_capacity(&self, options: MailboxOptions) -> Option<usize> {
    match options.capacity {
      QueueSize::Limitless => self.capacity,
      QueueSize::Limited(value) => Some(value),
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
    = TestQueue<M>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    let queue = MpscQueue::new(SharedBackendHandle::new(capacity));
    let signal = TestSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
