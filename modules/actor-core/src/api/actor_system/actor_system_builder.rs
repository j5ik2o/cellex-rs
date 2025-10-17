use crate::api::actor_runtime::ActorRuntime;
use crate::api::actor_runtime::MailboxQueueOf;
use crate::api::actor_runtime::MailboxSignalOf;
use crate::api::actor_system::ActorSystem;
use crate::api::actor_system::ActorSystemConfig;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use cellex_utils_core_rs::Element;
use core::marker::PhantomData;

/// Builder that constructs an [`ActorSystem`] by applying configuration overrides on top of a runtime preset.
pub struct ActorSystemBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  actor_runtime: R,
  config: ActorSystemConfig<R>,
  _marker: PhantomData<U>,
}

impl<U, R> ActorSystemBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
{
  /// Creates a new builder with default configuration.
  #[must_use]
  pub fn new(actor_runtime: R) -> Self {
    Self {
      actor_runtime,
      config: ActorSystemConfig::default(),
      _marker: PhantomData,
    }
  }

  /// Returns a reference to the runtime preset owned by the builder.
  #[must_use]
  pub fn actor_runtime(&self) -> &R {
    &self.actor_runtime
  }

  /// Returns a mutable reference to the runtime preset.
  pub fn actor_runtime_mut(&mut self) -> &mut R {
    &mut self.actor_runtime
  }

  /// Returns a reference to the configuration being accumulated.
  #[must_use]
  pub fn config(&self) -> &ActorSystemConfig<R> {
    &self.config
  }

  /// Returns a mutable reference to the configuration being accumulated.
  pub fn config_mut(&mut self) -> &mut ActorSystemConfig<R> {
    &mut self.config
  }

  /// Replaces the configuration with the provided value.
  #[must_use]
  pub fn with_config(mut self, config: ActorSystemConfig<R>) -> Self {
    self.config = config;
    self
  }

  /// Applies in-place configuration updates using the given closure.
  #[must_use]
  pub fn configure<F>(mut self, configure: F) -> Self
  where
    F: FnOnce(&mut ActorSystemConfig<R>), {
    configure(&mut self.config);
    self
  }

  /// Consumes the builder and constructs an [`ActorSystem`].
  #[must_use]
  pub fn build(self) -> ActorSystem<U, R> {
    ActorSystem::new_with_actor_runtime(self.actor_runtime, self.config)
  }
}
