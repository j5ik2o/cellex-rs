use core::marker::PhantomData;

#[cfg(feature = "embedded_rc")]
use cellex_actor_core_rs::api::mailbox::SingleThread;
#[cfg(not(feature = "embedded_rc"))]
use cellex_actor_core_rs::api::mailbox::ThreadSafe;
use cellex_actor_core_rs::api::mailbox::{
  queue_mailbox::QueueMailbox, MailboxFactory, MailboxOptions, MailboxPair, QueueMailboxProducer,
};
use cellex_utils_embedded_rs::Element;

use super::{
  local_mailbox_sender::LocalMailboxSender, local_mailbox_type::LocalMailbox, local_queue::LocalQueue,
  local_signal::LocalSignal,
};

/// Factory that creates local actor runtime.
///
/// Creates mailbox pairs for embedded or single-threaded environments.
#[derive(Clone, Debug, Default)]
pub struct LocalMailboxRuntime {
  _marker: PhantomData<()>,
}

impl LocalMailboxRuntime {
  /// Creates a new `LocalMailboxRuntime`.
  ///
  /// # Returns
  ///
  /// A new factory instance
  #[must_use]
  pub const fn new() -> Self {
    Self { _marker: PhantomData }
  }

  /// Creates a mailbox pair with the specified options.
  ///
  /// # Arguments
  ///
  /// * `options` - Mailbox configuration options
  ///
  /// # Returns
  ///
  /// A tuple of (receiver mailbox, sender handle)
  #[must_use]
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (LocalMailbox<M>, LocalMailboxSender<M>)
  where
    M: Element, {
    let (mailbox, sender) = self.build_mailbox::<M>(options);
    (LocalMailbox { inner: mailbox }, LocalMailboxSender { inner: sender })
  }

  /// Creates an unbounded mailbox pair.
  ///
  /// # Returns
  ///
  /// A tuple of (receiver mailbox, sender handle)
  #[must_use]
  pub fn unbounded<M>(&self) -> (LocalMailbox<M>, LocalMailboxSender<M>)
  where
    M: Element, {
    self.mailbox(MailboxOptions::unbounded())
  }
}

impl MailboxFactory for LocalMailboxRuntime {
  #[cfg(feature = "embedded_rc")]
  type Concurrency = SingleThread;
  #[cfg(not(feature = "embedded_rc"))]
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
    = LocalQueue<M>
  where
    M: Element;
  type Signal = LocalSignal;

  fn build_mailbox<M>(&self, _options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let queue = LocalQueue::new();
    let signal = LocalSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
