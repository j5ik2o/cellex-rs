use cellex_actor_core_rs::api::mailbox::{
  queue_mailbox::{LegacyQueueDriver, QueueMailbox},
  MailboxFactory, MailboxOptions, MailboxPair, QueueMailboxProducer, ThreadSafe,
};
use cellex_utils_std_rs::Element;

use super::{
  notify_signal::NotifySignal,
  tokio_mailbox_impl::TokioMailbox,
  tokio_mailbox_sender::TokioMailboxSender,
  tokio_queue::{create_tokio_queue, TokioQueue},
};

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
    = LegacyQueueDriver<TokioQueue<M>>
  where
    M: Element;
  type Signal = NotifySignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let queue = LegacyQueueDriver::new(create_tokio_queue::<M>(options.capacity));
    let signal = NotifySignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
