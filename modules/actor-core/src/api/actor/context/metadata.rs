use super::{Context, ContextLogger};
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::internal::context::ActorContext;
use crate::{ActorId, ActorPath, Extension, ExtensionId, Extensions, MetadataStorageMode, Supervisor};
use crate::{DynMessage, MailboxRuntime};
use cellex_utils_core_rs::Element;

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, crate::api::mailbox::PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Gets the metadata accompanying the current message.
  #[must_use]
  pub fn message_metadata(&self) -> Option<&crate::MessageMetadata<MailboxConcurrencyOf<R>>> {
    self.metadata.as_ref()
  }

  pub(crate) fn with_metadata(
    inner: &'r mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>,
    metadata: crate::MessageMetadata<MailboxConcurrencyOf<R>>,
  ) -> Self {
    let extensions = inner.extensions();
    Self {
      inner,
      metadata: Some(metadata),
      extensions,
      _marker: core::marker::PhantomData,
    }
  }

  /// Returns the shared extension registry.
  #[must_use]
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Applies the provided closure to the extension identified by `id`.
  pub fn extension<E, F, T>(&self, id: ExtensionId, f: F) -> Option<T>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> T, {
    self.extensions.with::<E, _, _>(id, f)
  }

  /// Gets the actor ID of this actor.
  #[must_use]
  pub fn actor_id(&self) -> ActorId {
    self.inner.actor_id()
  }

  /// Gets the actor path of this actor.
  #[must_use]
  pub fn actor_path(&self) -> &ActorPath {
    self.inner.actor_path()
  }

  /// Gets the list of actor IDs watching this actor.
  #[must_use]
  pub fn watchers(&self) -> &[ActorId] {
    self.inner.watchers()
  }

  /// Gets the logger for this actor.
  #[must_use]
  pub fn log(&self) -> ContextLogger {
    ContextLogger::new(self.actor_id(), self.actor_path())
  }

  /// Registers a watcher.
  pub fn register_watcher(&mut self, watcher: ActorId) {
    self.inner.register_watcher(watcher);
  }

  /// Unregisters a watcher.
  pub fn unregister_watcher(&mut self, watcher: ActorId) {
    self.inner.unregister_watcher(watcher);
  }

  /// Gets a mutable reference to the internal context.
  pub fn inner(&mut self) -> &mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>> {
    self.inner
  }
}
