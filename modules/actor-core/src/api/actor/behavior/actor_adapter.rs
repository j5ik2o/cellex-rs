use alloc::boxed::Box;
use core::mem;

use cellex_utils_core_rs::{sync::ArcShared, Element};

use super::{Behavior, BehaviorDirective, Signal, SignalFn, SupervisorStrategyConfig, SystemHandlerFn};
use crate::{
  api::{
    actor::{actor_context::ActorContext, actor_failure::ActorFailure},
    actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf},
    actor_system::map_system::MapSystemShared,
    mailbox::messages::{PriorityEnvelope, SystemMessage},
    messaging::{MessageEnvelope, MetadataStorageMode},
  },
  shared::messaging::AnyMessage,
};

/// Adapter that bridges Behavior to untyped runtime.
pub struct ActorAdapter<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  behavior_factory:          ArcShared<dyn Fn() -> Behavior<U, AR> + 'static>,
  pub(super) behavior:       Behavior<U, AR>,
  pub(super) system_handler: Option<Box<SystemHandlerFn<U, AR>>>,
}

impl<U, AR> ActorAdapter<U, AR>
where
  U: Element,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  MailboxConcurrencyOf<AR>: MetadataStorageMode,
{
  /// Creates a new `ActorAdapter`.
  ///
  /// # Errors
  /// None. This constructor cannot fail.
  pub fn new<S>(behavior_factory: ArcShared<dyn Fn() -> Behavior<U, AR> + 'static>, system_handler: Option<S>) -> Self
  where
    S: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, U, AR>, SystemMessage) + 'static, {
    let behavior = behavior_factory();
    Self {
      behavior_factory,
      behavior,
      system_handler: system_handler.map(|h| Box::new(h) as Box<SystemHandlerFn<U, AR>>),
    }
  }

  /// Processes a user message.
  ///
  /// # Errors
  /// Returns [`ActorFailure`] when behavior transitions fail or remain in setup state.
  pub fn handle_user(&mut self, ctx: &mut ActorContext<'_, '_, U, AR>, message: U) -> Result<(), ActorFailure> {
    self.ensure_initialized(ctx)?;
    while matches!(self.behavior, Behavior::Setup { .. }) {
      self.ensure_initialized(ctx)?;
    }
    match &mut self.behavior {
      | Behavior::Receive(state) => match state.handle(ctx, message)? {
        | BehaviorDirective::Same => {},
        | BehaviorDirective::Become(next) => self.transition(next, ctx)?,
      },
      | Behavior::Stopped => {},
      | Behavior::Setup { .. } => {
        return Err(ActorFailure::from_message("behavior remained in setup state"));
      },
    }
    Ok(())
  }

  /// Processes a system message.
  ///
  /// # Errors
  /// Returns [`ActorFailure`] when behavior transitions fail or remain in setup state.
  pub fn handle_system(
    &mut self,
    ctx: &mut ActorContext<'_, '_, U, AR>,
    message: SystemMessage,
  ) -> Result<(), ActorFailure> {
    self.ensure_initialized(ctx)?;
    while matches!(self.behavior, Behavior::Setup { .. }) {
      self.ensure_initialized(ctx)?;
    }
    if matches!(message, SystemMessage::Stop) {
      self.transition(Behavior::stopped(), ctx)?;
    } else if matches!(message, SystemMessage::Restart) {
      self.behavior = (self.behavior_factory)();
      self.ensure_initialized(ctx)?;
    }
    if matches!(self.behavior, Behavior::Setup { .. }) {
      return Err(ActorFailure::from_message("behavior remained in setup state"));
    }
    if let Some(handler) = self.system_handler.as_mut() {
      handler(ctx, message);
    }
    Ok(())
  }

  /// Creates a SystemMessage mapper for Guardian/Scheduler.
  #[must_use]
  pub fn create_map_system() -> MapSystemShared<AnyMessage> {
    MapSystemShared::new(|sys| AnyMessage::new(MessageEnvelope::<U>::System(sys)))
  }

  /// Gets supervisor configuration (internal API).
  pub(crate) fn supervisor_config(&self) -> SupervisorStrategyConfig {
    self.behavior.supervisor_config()
  }

  fn ensure_initialized(&mut self, ctx: &mut ActorContext<'_, '_, U, AR>) -> Result<(), ActorFailure> {
    while matches!(self.behavior, Behavior::Setup { .. }) {
      let (init, signal) = match mem::replace(&mut self.behavior, Behavior::stopped()) {
        | Behavior::Setup { init, signal } => (init, signal),
        | other => {
          self.behavior = other;
          break;
        },
      };
      let next_signal = signal.clone();
      let next_behavior = if let Some(init) = init { init(ctx)? } else { Behavior::stopped() };
      self.behavior = next_behavior.attach_signal_arc(next_signal);
    }
    Ok(())
  }

  fn current_signal_handler(&self) -> Option<ArcShared<SignalFn<U, AR>>> {
    match &self.behavior {
      | Behavior::Receive(state) => state.signal_handler(),
      | Behavior::Setup { signal, .. } => signal.clone(),
      | Behavior::Stopped => None,
    }
  }

  #[allow(dead_code)]
  fn handle_signal(&mut self, ctx: &mut ActorContext<'_, '_, U, AR>, signal: Signal) -> Result<(), ActorFailure> {
    if let Some(handler) = self.current_signal_handler() {
      match handler(ctx, signal) {
        | BehaviorDirective::Same => {},
        | BehaviorDirective::Become(next) => self.transition(next, ctx)?,
      }
    }
    Ok(())
  }

  fn transition(&mut self, next: Behavior<U, AR>, ctx: &mut ActorContext<'_, '_, U, AR>) -> Result<(), ActorFailure> {
    let previous_handler = self.current_signal_handler();
    self.behavior = next;
    self.ensure_initialized(ctx)?;
    if matches!(self.behavior, Behavior::Stopped) {
      let mut handler = self.current_signal_handler();
      if handler.is_none() {
        handler = previous_handler;
      }
      if let Some(handler) = handler {
        match handler(ctx, Signal::PostStop) {
          | BehaviorDirective::Same => {},
          | BehaviorDirective::Become(next) => self.transition(next, ctx)?,
        }
      }
    }
    Ok(())
  }
}
