use core::marker::PhantomData;

use crate::runtime::mailbox::traits::{MailboxFactory, MailboxHandle, MailboxPair, MailboxProducer};
use crate::runtime::mailbox::MailboxOptions;
use crate::PriorityEnvelope;
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::Element;

/// Shared handle that can spawn priority mailboxes without exposing the underlying factory.
pub struct PriorityMailboxSpawnerHandle<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  factory: ArcShared<R>,
  _marker: PhantomData<M>,
}

impl<M, R> Clone for PriorityMailboxSpawnerHandle<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  fn clone(&self) -> Self {
    Self {
      factory: self.factory.clone(),
      _marker: PhantomData,
    }
  }
}

impl<M, R> PriorityMailboxSpawnerHandle<M, R>
where
  M: Element,
  R: MailboxFactory,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  /// Creates a new handle from an `ArcShared`-wrapped factory.
  #[must_use]
  pub fn new(factory: ArcShared<R>) -> Self {
    Self {
      factory,
      _marker: PhantomData,
    }
  }

  /// Spawns a priority mailbox using the underlying factory and provided options.
  #[must_use]
  pub fn spawn_mailbox(
    &self,
    options: MailboxOptions,
  ) -> MailboxPair<R::Mailbox<PriorityEnvelope<M>>, R::Producer<PriorityEnvelope<M>>> {
    self
      .factory
      .with_ref(|factory| factory.build_mailbox::<PriorityEnvelope<M>>(options))
  }

  /// Returns the shared factory handle.
  #[must_use]
  pub fn factory(&self) -> ArcShared<R> {
    self.factory.clone()
  }
}

impl<M, R> PriorityMailboxSpawnerHandle<M, R>
where
  M: Element,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Mailbox<PriorityEnvelope<M>>: MailboxHandle<PriorityEnvelope<M>>,
  R::Producer<PriorityEnvelope<M>>: MailboxProducer<PriorityEnvelope<M>>,
{
  /// Wraps a factory value in `ArcShared` and returns a spawner handle.
  #[must_use]
  pub fn from_factory(factory: R) -> Self {
    Self::new(ArcShared::new(factory))
  }
}
