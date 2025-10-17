use crate::internal::mailbox::test_support::common::TestQueue;
use crate::internal::mailbox::test_support::shared_backend_handle::SharedBackendHandle;
use crate::internal::mailbox::test_support::test_signal::TestSignal;
use crate::{MailboxOptions, MailboxPair, MailboxRuntime, QueueMailbox, QueueMailboxProducer, ThreadSafe};
use cellex_utils_core_rs::{Element, MpscQueue, QueueSize};

#[derive(Clone, Debug, Default)]
pub struct TestMailboxRuntime {
  capacity: Option<usize>,
}

impl TestMailboxRuntime {
  pub const fn new(capacity: Option<usize>) -> Self {
    Self { capacity }
  }

  pub const fn with_capacity_per_queue(capacity: usize) -> Self {
    Self::new(Some(capacity))
  }

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

impl MailboxRuntime for TestMailboxRuntime {
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
