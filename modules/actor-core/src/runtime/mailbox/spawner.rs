use core::marker::PhantomData;

use crate::runtime::mailbox::traits::MailboxPair;
use crate::runtime::mailbox::MailboxOptions;
use crate::runtime::mailbox::PriorityMailboxBuilder;
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::Element;

/// Shared handle that can spawn priority mailboxes without exposing the underlying factory.
pub struct PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>, {
  builder: ArcShared<B>,
  _marker: PhantomData<M>,
}

impl<M, B> Clone for PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>,
{
  fn clone(&self) -> Self {
    Self {
      builder: self.builder.clone(),
      _marker: PhantomData,
    }
  }
}

impl<M, B> PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>,
{
  /// Creates a new handle from an `ArcShared`-wrapped factory.
  #[must_use]
  pub fn new(builder: ArcShared<B>) -> Self {
    Self {
      builder,
      _marker: PhantomData,
    }
  }

  /// Spawns a priority mailbox using the underlying factory and provided options.
  #[must_use]
  pub fn spawn_mailbox(
    &self,
    options: MailboxOptions,
  ) -> MailboxPair<<B as PriorityMailboxBuilder<M>>::Mailbox, <B as PriorityMailboxBuilder<M>>::Producer> {
    self.builder.with_ref(|builder| builder.build_priority_mailbox(options))
  }

  /// Returns the shared builder handle.
  #[must_use]
  pub fn builder(&self) -> ArcShared<B> {
    self.builder.clone()
  }
}

impl<M, B> PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>,
{
  /// Wraps a builder value in `ArcShared` and returns a spawner handle.
  #[must_use]
  pub fn from_builder(builder: B) -> Self {
    Self::new(ArcShared::new(builder))
  }
}

impl<M, R> PriorityMailboxSpawnerHandle<M, R>
where
  M: Element,
  R: PriorityMailboxBuilder<M> + Clone,
{
  /// Wraps a factory implementing [`PriorityMailboxBuilder`] and returns a spawner handle.
  #[must_use]
  pub fn from_factory(factory: R) -> Self {
    Self::from_builder(factory)
  }
}
