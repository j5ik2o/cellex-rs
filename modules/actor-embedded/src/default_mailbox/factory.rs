use core::marker::PhantomData;

use cellex_actor_core_rs::{
  api::mailbox::{
    queue_mailbox::{build_mailbox_queue, MailboxQueueConfig, QueueMailbox, SystemMailboxQueue},
    QueueMailboxProducer, ThreadSafe,
  },
  shared::mailbox::{MailboxFactory, MailboxOptions, MailboxPair},
};
use cellex_utils_core_rs::collections::Element;
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{default_mailbox_impl::DefaultMailbox, sender::DefaultMailboxSender, signal::DefaultSignal};

/// Factory for constructing [`DefaultMailbox`] instances.
pub struct DefaultMailboxFactory<RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  RM: RawMutex, {
  _marker: PhantomData<RM>,
}

impl<RM> Clone for DefaultMailboxFactory<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self::new()
  }
}

impl<RM> Default for DefaultMailboxFactory<RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<RM> DefaultMailboxFactory<RM>
where
  RM: RawMutex,
{
  /// Creates a new mailbox factory.
  pub const fn new() -> Self {
    Self { _marker: PhantomData }
  }

  /// Builds a mailbox using the supplied options.
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (DefaultMailbox<M, RM>, DefaultMailboxSender<M, RM>)
  where
    M: Element, {
    let (mailbox, sender) = self.build_mailbox::<M>(options);
    (DefaultMailbox { inner: mailbox }, DefaultMailboxSender { inner: sender })
  }

  /// Builds an unbounded mailbox.
  pub fn unbounded<M>(&self) -> (DefaultMailbox<M, RM>, DefaultMailboxSender<M, RM>)
  where
    M: Element, {
    self.mailbox(MailboxOptions::unbounded())
  }
}

impl<RM> MailboxFactory for DefaultMailboxFactory<RM>
where
  RM: RawMutex,
{
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
    = SystemMailboxQueue<M>
  where
    M: Element;
  type Signal = DefaultSignal<RM>;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let base = build_mailbox_queue::<M>(MailboxQueueConfig::default());
    let queue = SystemMailboxQueue::new(base, options.priority_capacity_limit());
    let signal = DefaultSignal::new();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
