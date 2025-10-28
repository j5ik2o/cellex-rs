use cellex_utils_core_rs::collections::Element;

use crate::shared::mailbox::{
  messages::PriorityEnvelope, MailboxConsumer, MailboxFactory, MailboxOptions, MailboxPair, MailboxProducer,
  MailboxSignal,
};

/// Builder abstraction specialised for priority mailboxes.
///
/// This trait extracts the responsibility of constructing priority mailboxes from
/// [`MailboxFactory`] so that the scheduler layer does not depend on concrete factories.
pub trait PriorityMailboxBuilder<M>: Clone
where
  M: Element, {
  /// Signal type used by the mailbox.
  type Signal: MailboxSignal;
  /// Mailbox consumer that stores priority envelopes.
  type Mailbox: MailboxConsumer<PriorityEnvelope<M>, Signal = Self::Signal> + Clone;
  /// Producer type that enqueues messages into the mailbox.
  type Producer: MailboxProducer<PriorityEnvelope<M>> + Clone;

  /// Builds a priority mailbox using the provided options.
  fn build_priority_mailbox(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox, Self::Producer>;

  /// Builds a priority mailbox using default options.
  #[allow(dead_code)]
  fn build_default_priority_mailbox(&self) -> MailboxPair<Self::Mailbox, Self::Producer> {
    self.build_priority_mailbox(MailboxOptions::default())
  }
}

impl<M, MF> PriorityMailboxBuilder<M> for MF
where
  M: Element,
  MF: MailboxFactory + Clone,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
{
  type Mailbox = MF::Mailbox<PriorityEnvelope<M>>;
  type Producer = MF::Producer<PriorityEnvelope<M>>;
  type Signal = MF::Signal;

  fn build_priority_mailbox(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox, Self::Producer> {
    MailboxFactory::build_mailbox::<PriorityEnvelope<M>>(self, options)
  }

  fn build_default_priority_mailbox(&self) -> MailboxPair<Self::Mailbox, Self::Producer> {
    MailboxFactory::build_default_mailbox::<PriorityEnvelope<M>>(self)
  }
}
