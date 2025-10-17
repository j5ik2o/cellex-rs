use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::internal::actor_system::InternalProps;
use crate::internal::context::ActorContext;
use crate::internal::message::take_metadata;
use crate::MailboxOptions;
use crate::{DynMessage, MetadataStorageMode};
use crate::{MailboxRuntime, Supervisor};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

use super::{ActorAdapter, ActorFailure, Behavior, Context};
use crate::api::actor::behavior::SupervisorStrategyConfig;
use crate::MessageEnvelope;
use core::marker::PhantomData;
use spin::Mutex;

/// Properties that hold configuration for actor spawning.
///
/// Includes actor behavior, mailbox settings, supervisor strategy, and more.
pub struct Props<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode, {
  inner: InternalProps<DynMessage, MailboxOf<R>>,
  _marker: PhantomData<U>,
  supervisor: SupervisorStrategyConfig,
}

impl<U, R> Props<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxOf<R>: MailboxRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Creates a new `Props` with the specified message handler.
  ///
  /// # Arguments
  /// * `handler` - Handler function to process user messages
  pub fn new<F>(handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<(), ActorFailure> + 'static, {
    let handler_cell = ArcShared::new(Mutex::new(handler));
    Self::with_behavior({
      let handler_cell = handler_cell.clone();
      move || {
        let handler_cell = handler_cell.clone();
        Behavior::stateless(move |ctx: &mut Context<'_, '_, U, R>, msg: U| {
          let mut guard = handler_cell.lock();
          (guard)(ctx, msg)
        })
      }
    })
  }

  /// Creates a new `Props` with the specified Behavior factory.
  ///
  /// # Arguments
  /// * `behavior_factory` - Factory function that generates actor behavior
  pub fn with_behavior<F>(behavior_factory: F) -> Self
  where
    F: Fn() -> Behavior<U, R> + 'static, {
    Self::with_behavior_and_system::<_, fn(&mut Context<'_, '_, U, R>, SystemMessage)>(behavior_factory, None)
  }

  /// Creates a new `Props` with user message handler and system message handler.
  ///
  /// # Arguments
  /// * `user_handler` - Handler function to process user messages
  /// * `system_handler` - Handler function to process system messages (optional)
  pub fn with_system_handler<F, G>(user_handler: F, system_handler: Option<G>) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<(), ActorFailure> + 'static,
    G: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, SystemMessage) + 'static, {
    let handler_cell = ArcShared::new(Mutex::new(user_handler));
    Self::with_behavior_and_system(
      {
        let handler_cell = handler_cell.clone();
        move || {
          let handler_cell = handler_cell.clone();
          Behavior::stateless(move |ctx: &mut Context<'_, '_, U, R>, msg: U| {
            let mut guard = handler_cell.lock();
            (guard)(ctx, msg)
          })
        }
      },
      system_handler,
    )
  }

  /// Creates a new `Props` with Behavior factory and system message handler.
  ///
  /// The most flexible way to create `Props`, allowing specification of both behavior and system message handler.
  ///
  /// # Arguments
  /// * `behavior_factory` - Factory function that generates actor behavior
  /// * `system_handler` - Handler function to process system messages (optional)
  pub fn with_behavior_and_system<F, S>(behavior_factory: F, system_handler: Option<S>) -> Self
  where
    F: Fn() -> Behavior<U, R> + 'static,
    S: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, SystemMessage) + 'static, {
    Self::with_behavior_and_system_with_options(MailboxOptions::default(), behavior_factory, system_handler)
  }

  fn with_behavior_and_system_with_options<F, S>(
    options: MailboxOptions,
    behavior_factory: F,
    system_handler: Option<S>,
  ) -> Self
  where
    F: Fn() -> Behavior<U, R> + 'static,
    S: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, SystemMessage) + 'static, {
    let behavior_factory =
      ArcShared::new(behavior_factory).into_dyn(|factory| factory as &(dyn Fn() -> Behavior<U, R> + 'static));
    let mut adapter = ActorAdapter::new(behavior_factory.clone(), system_handler);
    let map_system = ActorAdapter::<U, R>::create_map_system();
    let supervisor = adapter.supervisor_config();

    let handler = move |ctx: &mut ActorContext<'_, DynMessage, MailboxOf<R>, dyn Supervisor<DynMessage>>,
                        message: DynMessage|
          -> Result<(), ActorFailure> {
      let Ok(envelope) = message.downcast::<MessageEnvelope<U>>() else {
        panic!("unexpected message type delivered to typed handler");
      };
      match envelope {
        MessageEnvelope::User(user) => {
          let (message, metadata_key) = user.into_parts();
          let metadata = metadata_key
            .and_then(take_metadata::<MailboxConcurrencyOf<R>>)
            .unwrap_or_default();
          let mut typed_ctx = Context::with_metadata(ctx, metadata);
          adapter.handle_user(&mut typed_ctx, message)?;
          Ok(())
        }
        MessageEnvelope::System(message) => {
          let mut typed_ctx = Context::new(ctx);
          adapter.handle_system(&mut typed_ctx, message)?;
          Ok(())
        }
      }
    };

    let inner = InternalProps::new(options, map_system, handler);
    Self {
      inner,
      _marker: PhantomData,
      supervisor,
    }
  }

  /// Overrides the mailbox options for this `Props`.
  #[must_use]
  pub fn with_mailbox_options(mut self, options: MailboxOptions) -> Self {
    self.inner.options = options;
    self
  }

  /// Decomposes into internal properties and supervisor configuration (internal API).
  ///
  /// # Returns
  /// Tuple of `(InternalProps, SupervisorStrategyConfig)`
  pub(crate) fn into_parts(self) -> (InternalProps<DynMessage, MailboxOf<R>>, SupervisorStrategyConfig) {
    (self.inner, self.supervisor)
  }
}
