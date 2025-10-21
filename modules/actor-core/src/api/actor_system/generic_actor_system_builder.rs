use core::marker::PhantomData;

use cellex_utils_core_rs::Element;

use crate::{
  api::{
    actor_runtime::{ActorRuntime, MailboxQueueOf, MailboxSignalOf},
    actor_system::{GenericActorSystem, GenericActorSystemConfig},
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Builder that constructs a [`GenericActorSystem`] by applying configuration overrides on top of a
/// runtime preset.
pub struct GenericActorSystemBuilder<U, AR>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  actor_runtime: AR,
  config:        GenericActorSystemConfig<AR>,
  _marker:       PhantomData<U>,
}

impl<U, AR> GenericActorSystemBuilder<U, AR>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
{
  /// Creates a new builder with default configuration.
  #[must_use]
  pub fn new(actor_runtime: AR) -> Self {
    Self { actor_runtime, config: GenericActorSystemConfig::default(), _marker: PhantomData }
  }

  /// Returns a reference to the runtime preset owned by the builder.
  #[must_use]
  pub const fn actor_runtime(&self) -> &AR {
    &self.actor_runtime
  }

  /// Returns a mutable reference to the runtime preset.
  pub const fn actor_runtime_mut(&mut self) -> &mut AR {
    &mut self.actor_runtime
  }

  /// Returns a reference to the configuration being accumulated.
  #[must_use]
  pub const fn config(&self) -> &GenericActorSystemConfig<AR> {
    &self.config
  }

  /// Returns a mutable reference to the configuration being accumulated.
  pub const fn config_mut(&mut self) -> &mut GenericActorSystemConfig<AR> {
    &mut self.config
  }

  /// Replaces the configuration with the provided value.
  #[must_use]
  pub fn with_config(mut self, config: GenericActorSystemConfig<AR>) -> Self {
    self.config = config;
    self
  }

  /// Applies in-place configuration updates using the given closure.
  #[must_use]
  pub fn configure<F>(mut self, configure: F) -> Self
  where
    F: FnOnce(&mut GenericActorSystemConfig<AR>), {
    configure(&mut self.config);
    self
  }

  /// Consumes the builder and constructs an [`GenericActorSystem`].
  #[must_use]
  pub fn build(self) -> GenericActorSystem<U, AR> {
    GenericActorSystem::new_with_actor_runtime(self.actor_runtime, self.config)
  }
}
