use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::context::ActorContext;
use crate::MailboxRuntime;
use crate::{DynMessage, Extensions, MetadataStorageMode, Supervisor};
use cellex_utils_core_rs::Element;
use core::marker::PhantomData;

mod adapter;
mod logger;
mod messaging;
mod metadata;
mod receive_timeout;
mod spawn;

pub use adapter::{MessageAdapterRef, MessageMetadataResponder};
pub use logger::{ContextLogLevel, ContextLogger};

#[cfg(target_has_atomic = "ptr")]
pub(super) type AdapterFn<Ext, U> = dyn Fn(Ext) -> U + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(super) type AdapterFn<Ext, U> = dyn Fn(Ext) -> U;

/// Typed actor execution context wrapper.
/// 'r: lifetime of the mutable reference to ActorContext
/// 'ctx: lifetime parameter of ActorContext itself
pub struct Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode, {
  pub(super) inner: &'r mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>,
  pub(super) metadata: Option<crate::MessageMetadata<MailboxConcurrencyOf<R>>>,
  pub(super) extensions: Extensions,
  pub(super) _marker: PhantomData<U>,
}

/// Type alias for context during setup.
pub type SetupContext<'ctx, U, R> = Context<'ctx, 'ctx, U, R>;

impl<'r, 'ctx, U, R> Context<'r, 'ctx, U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  pub(super) fn new(inner: &'r mut ActorContext<'ctx, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>) -> Self {
    let extensions = inner.extensions();
    Self {
      inner,
      metadata: None,
      extensions,
      _marker: PhantomData,
    }
  }
}
