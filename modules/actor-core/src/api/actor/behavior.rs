use alloc::boxed::Box;

use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::PriorityEnvelope;
use crate::{DynMessage, MetadataStorageMode};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

use super::{ActorFailure, Context};

mod actor_adapter;
mod behavior_directive;
mod behavior_state;
mod behaviors;
mod dyn_supervisor;
mod fixed_directive_supervisor;
mod supervise_builder;
mod supervisor_strategy;
mod supervisor_strategy_config;

pub use actor_adapter::ActorAdapter;
use behavior_directive::BehaviorDirective;
use behavior_state::BehaviorState;
#[allow(unused_imports)]
pub use behaviors::Behaviors;
#[allow(unused_imports)]
pub use supervisor_strategy::SupervisorStrategy;
pub use supervisor_strategy_config::SupervisorStrategyConfig;

pub(super) type ReceiveFn<U, R> =
  dyn for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<BehaviorDirective<U, R>, ActorFailure> + 'static;
pub(super) type SystemHandlerFn<U, R> =
  dyn for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, crate::api::mailbox::SystemMessage) + 'static;
pub(super) type SignalFn<U, R> =
  dyn for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>, Signal) -> BehaviorDirective<U, R> + 'static;
pub(super) type SetupFn<U, R> =
  dyn for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Result<Behavior<U, R>, ActorFailure> + 'static;

/// Actor lifecycle signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
  /// Signal sent after actor stops
  PostStop,
}

/// Typed Behavior representation. Equivalent to Akka/Pekko Typed's `Behavior`.
pub enum Behavior<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  /// Message receiving state
  Receive(BehaviorState<U, R>),
  /// Execute setup processing to generate Behavior
  Setup {
    /// Initialization callback (returns Result).
    init: Option<ArcShared<SetupFn<U, R>>>,
    /// Signal handler retained across behavior transitions.
    signal: Option<ArcShared<SignalFn<U, R>>>,
  },
  /// Stopped state
  Stopped,
}

impl<U, R> Behavior<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Constructs a `Behavior` with specified message receive handler.
  #[must_use]
  pub fn receive<F>(mut handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<BehaviorDirective<U, R>, ActorFailure> + 'static,
  {
    Self::Receive(BehaviorState::new(
      Box::new(move |ctx, msg| handler(ctx, msg)),
      SupervisorStrategyConfig::default(),
    ))
  }

  /// Constructs Behavior with a simple stateless handler.
  #[must_use]
  pub fn stateless<F>(mut handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<(), ActorFailure> + 'static, {
    Self::receive(move |ctx, msg| {
      handler(ctx, msg)?;
      Ok(BehaviorDirective::Same)
    })
  }

  /// Constructs Behavior with a handler that receives only the message.
  #[must_use]
  pub fn receive_message<F>(mut handler: F) -> Self
  where
    F: FnMut(U) -> Result<BehaviorDirective<U, R>, ActorFailure> + 'static, {
    Self::receive(move |_, msg| handler(msg))
  }

  /// Creates a Behavior in stopped state.
  #[must_use]
  pub const fn stopped() -> Self {
    Self::Stopped
  }

  /// Executes setup processing to generate Behavior.
  pub fn setup<F>(init: F) -> Self
  where
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Result<Behavior<U, R>, ActorFailure> + 'static, {
    let handler = ArcShared::new(init).into_dyn(|inner| inner as &SetupFn<U, R>);
    Self::Setup {
      init: Some(handler),
      signal: None,
    }
  }

  /// Gets supervisor configuration (internal API).
  pub(crate) fn supervisor_config(&self) -> SupervisorStrategyConfig {
    match self {
      Behavior::Receive(state) => state.supervisor.clone(),
      Behavior::Setup { .. } | Behavior::Stopped => SupervisorStrategyConfig::default(),
    }
  }

  /// Adds a signal handler.
  pub fn receive_signal<F>(self, handler: F) -> Self
  where
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>, Signal) -> BehaviorDirective<U, R> + 'static, {
    let handler = ArcShared::new(handler).into_dyn(|inner| inner as &SignalFn<U, R>);
    self.attach_signal_arc(Some(handler))
  }

  pub(super) fn attach_signal_arc(mut self, handler: Option<ArcShared<SignalFn<U, R>>>) -> Self {
    if let Some(handler) = handler {
      match &mut self {
        Behavior::Receive(state) => {
          state.set_signal_handler(handler);
        }
        Behavior::Setup { signal, .. } => {
          *signal = Some(handler);
        }
        Behavior::Stopped => {}
      }
    }
    self
  }
}
