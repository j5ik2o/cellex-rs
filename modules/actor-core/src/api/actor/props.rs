use core::marker::PhantomData;

use cellex_utils_core_rs::{sync::ArcShared, Element};
use spin::Mutex;

use super::{
  actor_failure::ActorFailure,
  behavior::{ActorAdapter, Behavior},
  context::Context,
};
use crate::{
  api::{
    actor::{actor_context::ActorContext, behavior::SupervisorStrategyConfig},
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    mailbox::{MailboxFactory, MailboxOptions, PriorityEnvelope, SystemMessage},
    messaging::{DynMessage, MessageEnvelope, MetadataStorageMode},
    supervision::supervisor::Supervisor,
  },
  internal::actor::InternalProps,
};

/// Properties that hold configuration for actor spawning.
///
/// Includes actor behavior, mailbox settings, supervisor strategy, and more.
pub struct Props<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  inner:      InternalProps<DynMessage, MailboxOf<AR>>,
  _marker:    PhantomData<U>,
  supervisor: SupervisorStrategyConfig,
}

impl<U, AR> Props<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Creates a new `Props` with the specified message handler.
  ///
  /// # Arguments
  /// * `handler` - Handler function to process user messages
  pub fn new<F>(handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, AR>, U) -> Result<(), ActorFailure> + 'static, {
    let handler_cell = ArcShared::new(Mutex::new(handler));
    Self::with_behavior({
      let handler_cell = handler_cell.clone();
      move || {
        let handler_cell = handler_cell.clone();
        Behavior::stateless(move |ctx: &mut Context<'_, '_, U, AR>, msg: U| {
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
    F: Fn() -> Behavior<U, AR> + 'static, {
    Self::with_behavior_and_system::<_, fn(&mut Context<'_, '_, U, AR>, SystemMessage)>(behavior_factory, None)
  }

  /// Creates a new `Props` with user message handler and system message handler.
  ///
  /// # Arguments
  /// * `user_handler` - Handler function to process user messages
  /// * `system_handler` - Handler function to process system messages (optional)
  pub fn with_system_handler<F, G>(user_handler: F, system_handler: Option<G>) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, AR>, U) -> Result<(), ActorFailure> + 'static,
    G: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    let handler_cell = ArcShared::new(Mutex::new(user_handler));
    Self::with_behavior_and_system(
      {
        let handler_cell = handler_cell.clone();
        move || {
          let handler_cell = handler_cell.clone();
          Behavior::stateless(move |ctx: &mut Context<'_, '_, U, AR>, msg: U| {
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
  /// The most flexible way to create `Props`, allowing specification of both behavior and system
  /// message handler.
  ///
  /// # Arguments
  /// * `behavior_factory` - Factory function that generates actor behavior
  /// * `system_handler` - Handler function to process system messages (optional)
  pub fn with_behavior_and_system<F, S>(behavior_factory: F, system_handler: Option<S>) -> Self
  where
    F: Fn() -> Behavior<U, AR> + 'static,
    S: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    Self::with_behavior_and_system_with_options(MailboxOptions::default(), behavior_factory, system_handler)
  }

  fn with_behavior_and_system_with_options<F, S>(
    options: MailboxOptions,
    behavior_factory: F,
    system_handler: Option<S>,
  ) -> Self
  where
    F: Fn() -> Behavior<U, AR> + 'static,
    S: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    let behavior_factory =
      ArcShared::new(behavior_factory).into_dyn(|factory| factory as &(dyn Fn() -> Behavior<U, AR> + 'static));
    let mut adapter = ActorAdapter::new(behavior_factory.clone(), system_handler);
    let map_system = ActorAdapter::<U, AR>::create_map_system();
    let supervisor = adapter.supervisor_config();

    let handler = move |ctx: &mut ActorContext<'_, DynMessage, MailboxOf<AR>, dyn Supervisor<DynMessage>>,
                        message: DynMessage|
          -> Result<(), ActorFailure> {
      let Ok(envelope) = message.downcast::<MessageEnvelope<U>>() else {
        panic!("unexpected message type delivered to typed handler");
      };
      match envelope {
        | MessageEnvelope::User(user) => {
          let (message, metadata) = user.into_parts::<MailboxConcurrencyOf<AR>>();
          let metadata = metadata.unwrap_or_default();
          let mut typed_ctx = Context::with_metadata(ctx, metadata);
          adapter.handle_user(&mut typed_ctx, message)?;
          Ok(())
        },
        | MessageEnvelope::System(message) => {
          let mut typed_ctx = Context::new(ctx);
          adapter.handle_system(&mut typed_ctx, message)?;
          Ok(())
        },
      }
    };

    let inner = InternalProps::new(options, map_system, handler);
    Self { inner, _marker: PhantomData, supervisor }
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
  pub(crate) fn into_parts(self) -> (InternalProps<DynMessage, MailboxOf<AR>>, SupervisorStrategyConfig) {
    (self.inner, self.supervisor)
  }
}
