use cellex_utils_core_rs::Element;

use super::{ActorSystem, ActorSystemConfig};
use crate::{
  api::{
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    guardian::GuardianStrategy,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Defines the builder surface responsible for constructing actor system instances.
pub trait ActorSystemBuilder<U, AR>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Self: Sized, {
  /// Guardian strategy associated with the produced system.
  type Strategy: GuardianStrategy<MailboxOf<AR>>;

  /// Configuration type accumulated by the builder.
  type Config: ActorSystemConfig<AR>;

  /// Actor system type constructed by the builder.
  type System: ActorSystem<U, AR, Self::Strategy>;

  /// Creates a new builder from a runtime preset.
  fn new(actor_runtime: AR) -> Self;

  /// Borrows the runtime preset.
  fn actor_runtime(&self) -> &AR;

  /// Borrows the runtime preset mutably.
  fn actor_runtime_mut(&mut self) -> &mut AR;

  /// Borrows the accumulated configuration.
  fn config(&self) -> &Self::Config;

  /// Borrows the accumulated configuration mutably.
  fn config_mut(&mut self) -> &mut Self::Config;

  /// Replaces the accumulated configuration.
  fn with_config(self, config: Self::Config) -> Self;

  /// Applies configuration updates in place.
  fn configure<F>(self, configure: F) -> Self
  where
    F: FnOnce(&mut Self::Config);

  /// Consumes the builder and creates an actor system.
  fn build(self) -> Self::System;
}
