use cellex_actor_core_rs::{
  api::mailbox::{
    queue_mailbox::{build_mailbox_queue, MailboxQueueConfig, QueueMailbox, SystemMailboxQueue},
    QueueMailboxProducer,
  },
  shared::mailbox::{MailboxFactory, MailboxOptions, MailboxPair},
};
use cellex_utils_core_rs::collections::{queue::QueueSize, Element};

use super::{notify_signal::NotifySignal, tokio_mailbox_impl::TokioMailbox, tokio_mailbox_sender::TokioMailboxSender};

type TokioQueueDriver<M> = SystemMailboxQueue<M>;
type TokioMailboxInner<M> = QueueMailbox<SystemMailboxQueue<M>, NotifySignal>;
type TokioMailboxProducer<M> = QueueMailboxProducer<SystemMailboxQueue<M>, NotifySignal>;

/// Factory that creates Tokio mailboxes.
///
/// Provides constructors for bounded and unbounded mailboxes.
/// CAUTION: keep the type name accurate and ensure the implementation matches it.
#[derive(Clone, Debug, Default)]
pub struct TokioMailboxFactory;

impl TokioMailboxFactory {
  /// Creates a mailbox with the specified options
  ///
  /// # Arguments
  /// * `options` - Configuration options for the mailbox
  ///
  /// # Returns
  /// A pair of mailbox and sender handle
  #[must_use]
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (TokioMailbox<M>, TokioMailboxSender<M>)
  where
    M: Element, {
    let (mailbox, sender) = self.build_mailbox::<M>(options);
    (TokioMailbox { inner: mailbox }, TokioMailboxSender { inner: sender })
  }

  /// Creates a bounded mailbox with the specified capacity
  ///
  /// # Arguments
  /// * `capacity` - Maximum capacity of the mailbox
  ///
  /// # Returns
  /// A pair of mailbox and sender handle
  #[must_use]
  pub fn with_capacity<M>(&self, capacity: usize) -> (TokioMailbox<M>, TokioMailboxSender<M>)
  where
    M: Element, {
    self.mailbox(MailboxOptions::with_capacity(capacity))
  }

  /// Creates an unbounded mailbox
  ///
  /// # Returns
  /// A pair of mailbox and sender handle
  #[must_use]
  pub fn unbounded<M>(&self) -> (TokioMailbox<M>, TokioMailboxSender<M>)
  where
    M: Element, {
    self.mailbox(MailboxOptions::unbounded())
  }
}

impl MailboxFactory for TokioMailboxFactory {
  type Concurrency = cellex_actor_core_rs::api::mailbox::ThreadSafe;
  type Mailbox<M>
    = TokioMailboxInner<M>
  where
    M: Element;
  type Producer<M>
    = TokioMailboxProducer<M>
  where
    M: Element;
  type Queue<M>
    = TokioQueueDriver<M>
  where
    M: Element;
  type Signal = NotifySignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let queue = {
      let capacity_size = match options.capacity {
        | QueueSize::Limitless | QueueSize::Limited(0) => QueueSize::limitless(),
        | QueueSize::Limited(capacity) => QueueSize::limited(capacity),
      };
      let config =
        MailboxQueueConfig::new(capacity_size, cellex_actor_core_rs::api::mailbox::MailboxOverflowPolicy::Block);
      let base = build_mailbox_queue::<M>(config);
      SystemMailboxQueue::new(base, options.priority_capacity_limit())
    };
    let signal = NotifySignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
