use crate::api::mailbox::mailbox_concurrency::MailboxConcurrency;
use crate::api::mailbox::mailbox_handle::MailboxHandle;
use crate::api::mailbox::mailbox_options::MailboxOptions;
use crate::api::mailbox::mailbox_producer::MailboxProducer;
use crate::api::mailbox::mailbox_signal::MailboxSignal;
use crate::api::mailbox::MailboxPair;
use crate::internal::message::MetadataStorageMode;
use cellex_utils_core_rs::{Element, QueueRw};

/// Factory trait for creating mailboxes.
///
/// Generates mailbox and queue implementations according to
/// specific async runtimes (Tokio, Async-std, etc.).
pub trait MailboxRuntime {
  /// Declares the concurrency mode for this factory.
  type Concurrency: MailboxConcurrency + MetadataStorageMode;

  /// Type of notification signal
  type Signal: MailboxSignal;

  /// Type of message queue
  type Queue<M>: QueueRw<M> + Clone
  where
    M: Element;

  /// Mailbox handle returned to the scheduler.
  type Mailbox<M>: MailboxHandle<M, Signal = Self::Signal> + Clone
  where
    M: Element;

  /// Producer handle used for enqueuing messages into the mailbox.
  type Producer<M>: MailboxProducer<M> + Clone
  where
    M: Element;

  /// Creates a mailbox with the specified options.
  ///
  /// # Arguments
  /// - `options`: Capacity settings for the mailbox
  ///
  /// # Returns
  /// Pair of `(mailbox, producer)`
  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element;

  /// Creates a mailbox with default settings.
  ///
  /// # Returns
  /// Pair of `(mailbox, producer)`
  fn build_default_mailbox<M>(&self) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    self.build_mailbox(MailboxOptions::default())
  }
}
