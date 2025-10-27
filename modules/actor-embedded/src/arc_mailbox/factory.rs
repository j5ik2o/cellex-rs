use core::marker::PhantomData;

use cellex_actor_core_rs::{
  api::mailbox::{
    queue_mailbox::{build_queue_driver, QueueDriverConfig, QueueMailbox},
    QueueMailboxProducer, ThreadSafe,
  },
  shared::mailbox::{MailboxFactory, MailboxOptions, MailboxPair},
};
use cellex_utils_core_rs::collections::Element;
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{
  arc_mailbox_impl::ArcMailbox, sender::ArcMailboxSender, signal::ArcSignal, sync_queue_handle::ArcSyncQueueDriver,
};

/// Factory for constructing [`ArcMailbox`] instances.
pub struct ArcMailboxFactory<RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  RM: RawMutex, {
  _marker: PhantomData<RM>,
}

impl<RM> Clone for ArcMailboxFactory<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self::new()
  }
}

impl<RM> Default for ArcMailboxFactory<RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<RM> ArcMailboxFactory<RM>
where
  RM: RawMutex,
{
  /// Creates a new mailbox factory.
  pub const fn new() -> Self {
    Self { _marker: PhantomData }
  }

  /// Builds a mailbox using the supplied options.
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (ArcMailbox<M, RM>, ArcMailboxSender<M, RM>)
  where
    M: Element, {
    let (mailbox, sender) = self.build_mailbox::<M>(options);
    (ArcMailbox { inner: mailbox }, ArcMailboxSender { inner: sender })
  }

  /// Builds an unbounded mailbox.
  pub fn unbounded<M>(&self) -> (ArcMailbox<M, RM>, ArcMailboxSender<M, RM>)
  where
    M: Element, {
    self.mailbox(MailboxOptions::unbounded())
  }
}

impl<RM> MailboxFactory for ArcMailboxFactory<RM>
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
    = ArcSyncQueueDriver<M, RM>
  where
    M: Element;
  type Signal = ArcSignal<RM>;

  fn build_mailbox<M>(&self, _options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let queue = ArcSyncQueueDriver::from_driver(build_queue_driver::<M>(QueueDriverConfig::default()));
    let signal = ArcSignal::new();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
