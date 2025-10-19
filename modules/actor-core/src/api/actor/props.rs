use core::marker::PhantomData;

use cellex_utils_core_rs::{sync::ArcShared, Element};
use spin::Mutex;

use super::{
  actor_context::ActorContext,
  actor_failure::ActorFailure,
  behavior::{ActorAdapter, Behavior},
};
use crate::{
  api::{
    actor::behavior::SupervisorStrategyConfig,
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    mailbox::{
      messages::{PriorityEnvelope, SystemMessage},
      MailboxFactory, MailboxOptions,
    },
    messaging::{AnyMessage, MetadataStorageMode},
  },
  internal::actor::{internal_props_from_adapter, InternalProps},
};

/// Properties that hold configuration for actor spawning.
///
/// Includes actor behavior, mailbox settings, supervisor strategy, and more.
pub struct Props<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode, {
  inner:      InternalProps<MailboxOf<AR>>,
  _marker:    PhantomData<U>,
  supervisor: SupervisorStrategyConfig,
}

impl<U, AR> Props<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxOf<AR>: MailboxFactory + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Creates a new `Props` with the specified message handler.
  ///
  /// # Arguments
  /// * `handler` - Handler function to process user messages
  pub fn new<F>(handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, U) -> Result<(), ActorFailure> + 'static, {
    let handler_cell = ArcShared::new(Mutex::new(handler));
    Self::with_behavior(move || {
      let handler_cell = handler_cell.clone();
      Behavior::stateless(move |ctx: &mut ActorContext<'_, '_, U, AR>, msg: U| {
        let mut guard = handler_cell.lock();
        (guard)(ctx, msg)
      })
    })
  }

  /// Creates a new `Props` with the specified Behavior factory.
  ///
  /// # Arguments
  /// * `behavior_factory` - Factory function that generates actor behavior
  pub fn with_behavior<F>(behavior_factory: F) -> Self
  where
    F: Fn() -> Behavior<U, AR> + 'static, {
    Self::with_behavior_and_system::<_, fn(&mut ActorContext<'_, '_, U, AR>, SystemMessage)>(behavior_factory, None)
  }

  /// Creates a new `Props` with user message handler and system message handler.
  ///
  /// # Arguments
  /// * `user_handler` - Handler function to process user messages
  /// * `system_handler` - Handler function to process system messages (optional)
  pub fn with_system_handler<F, G>(user_handler: F, system_handler: Option<G>) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, U) -> Result<(), ActorFailure> + 'static,
    G: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    let handler_cell = ArcShared::new(Mutex::new(user_handler));
    Self::with_behavior_and_system(
      move || {
        let handler_cell = handler_cell.clone();
        Behavior::stateless(move |ctx: &mut ActorContext<'_, '_, U, AR>, msg: U| {
          let mut guard = handler_cell.lock();
          (guard)(ctx, msg)
        })
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
    S: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    Self::with_behavior_and_system_with_options(MailboxOptions::default(), behavior_factory, system_handler)
  }

  fn with_behavior_and_system_with_options<F, S>(
    options: MailboxOptions,
    behavior_factory: F,
    system_handler: Option<S>,
  ) -> Self
  where
    F: Fn() -> Behavior<U, AR> + 'static,
    S: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    let behavior_factory =
      ArcShared::new(behavior_factory).into_dyn(|factory| factory as &(dyn Fn() -> Behavior<U, AR> + 'static));
    let adapter = ActorAdapter::new(behavior_factory.clone(), system_handler);
    let map_system = ActorAdapter::<U, AR>::create_map_system();
    let supervisor = adapter.supervisor_config();

    let inner = internal_props_from_adapter(options, map_system, adapter);
    Self { inner, _marker: PhantomData, supervisor }
  }

  /// Overrides the mailbox options for this `Props`.
  #[must_use]
  #[allow(clippy::missing_const_for_fn)]
  pub fn with_mailbox_options(mut self, options: MailboxOptions) -> Self {
    self.inner.options = options;
    self
  }

  /// Decomposes into internal properties and supervisor configuration (internal API).
  ///
  /// # Returns
  /// Tuple of `(InternalProps, SupervisorStrategyConfig)`
  pub(crate) fn into_parts(self) -> (InternalProps<MailboxOf<AR>>, SupervisorStrategyConfig) {
    (self.inner, self.supervisor)
  }
}
