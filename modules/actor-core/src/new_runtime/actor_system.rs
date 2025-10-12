//! Bridges the public new actor system API to the existing internal runtime.

use core::marker::PhantomData;

use cellex_utils_core_rs::{Element, Shared};

use crate::api::actor::ShutdownToken;
use crate::runtime::guardian::AlwaysRestart;
use crate::runtime::mailbox::traits::MailboxRuntime;
use crate::runtime::message::DynMessage;
use crate::runtime::system::{InternalActorSystem, InternalActorSystemSettings};
use crate::{Extensions, PriorityEnvelope};

use super::bundle::NewActorRuntimeBundle;
use super::mailbox::NewMailboxRuntime;
use super::runtime_parts::RuntimeParts;

/// Internal wrapper that keeps the legacy [`InternalActorSystem`] hidden behind the new API.
pub struct NewInternalActorSystem<M, B, Strat = AlwaysRestart>
where
  M: Element + 'static,
  B: NewActorRuntimeBundle,
  B::MailboxRuntime: NewMailboxRuntime + Clone + 'static,
  <B::MailboxRuntime as MailboxRuntime>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Signal: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  inner: InternalActorSystem<DynMessage, B::MailboxRuntime>,
  _phantom: PhantomData<(M, Strat)>,
}

impl<M, B, Strat> NewInternalActorSystem<M, B, Strat>
where
  M: Element + 'static,
  B: NewActorRuntimeBundle,
  B::MailboxRuntime: NewMailboxRuntime + Clone + 'static,
  <B::MailboxRuntime as MailboxRuntime>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Signal: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  /// Builds a new internal system by reusing existing runtime components from the bundle.
  #[must_use]
  pub fn from_bundle(bundle: &B) -> Self {
    Self::from_parts(bundle.runtime_parts())
  }

  /// Builds a new internal system from the provided runtime parts.
  #[must_use]
  pub fn from_parts(parts: RuntimeParts<B::MailboxRuntime>) -> Self {
    let runtime_shared = parts.mailbox_factory.with_ref(|factory| factory.runtime_shared());
    let runtime = runtime_shared.with_ref(|runtime| runtime.clone());
    let scheduler_builder = parts.scheduler_builder.with_ref(|builder| builder.builder());
    let settings = InternalActorSystemSettings {
      root_event_listener: parts.root_event_listener.clone(),
      root_escalation_handler: parts.root_escalation_handler.clone(),
      receive_timeout_factory: parts.resolve_receive_timeout_factory(),
      metrics_sink: parts.metrics_sink.clone(),
      extensions: parts.extensions.clone(),
    };
    let inner = InternalActorSystem::new_with_settings_and_builder(runtime, scheduler_builder, settings);
    Self {
      inner,
      _phantom: PhantomData,
    }
  }

  /// Returns an immutable reference to the underlying system.
  #[allow(dead_code)]
  pub(crate) fn inner(&self) -> &InternalActorSystem<DynMessage, B::MailboxRuntime> {
    &self.inner
  }

  /// Returns a mutable reference to the underlying system.
  #[allow(dead_code)]
  pub(crate) fn inner_mut(&mut self) -> &mut InternalActorSystem<DynMessage, B::MailboxRuntime> {
    &mut self.inner
  }

  /// Clones the extension registry associated with the system.
  #[must_use]
  pub fn extensions(&self) -> Extensions {
    self.inner.extensions()
  }
}

/// Public actor system entry point for the new runtime API.
pub struct NewActorSystem<U, B, Strat = AlwaysRestart>
where
  U: Element,
  B: NewActorRuntimeBundle,
  B::MailboxRuntime: NewMailboxRuntime + Clone + 'static,
  <B::MailboxRuntime as MailboxRuntime>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Signal: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  bundle: B,
  inner: NewInternalActorSystem<DynMessage, B, Strat>,
  shutdown: ShutdownToken,
  extensions: Extensions,
  _marker: PhantomData<U>,
}

impl<U, B, Strat> NewActorSystem<U, B, Strat>
where
  U: Element,
  B: NewActorRuntimeBundle,
  B::MailboxRuntime: NewMailboxRuntime + Clone + 'static,
  <B::MailboxRuntime as MailboxRuntime>::Queue<PriorityEnvelope<DynMessage>>: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Signal: Clone,
  <B::MailboxRuntime as MailboxRuntime>::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  /// Constructs a new actor system using the provided bundle.
  #[must_use]
  pub fn new(bundle: B) -> Self {
    let inner = NewInternalActorSystem::from_bundle(&bundle);
    let extensions = inner.extensions();
    Self {
      bundle,
      inner,
      shutdown: ShutdownToken::default(),
      extensions,
      _marker: PhantomData,
    }
  }

  /// Returns the shutdown token shared with the runner.
  #[must_use]
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.shutdown.clone()
  }

  /// Clones the extension registry managed by this system.
  #[must_use]
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Returns the bundle associated with this actor system.
  #[must_use]
  pub fn bundle(&self) -> &B {
    &self.bundle
  }

  /// Provides access to the internal system for advanced scenarios.
  pub fn internal(&self) -> &NewInternalActorSystem<DynMessage, B, Strat> {
    &self.inner
  }

  /// Provides mutable access to the internal system for advanced scenarios.
  pub fn internal_mut(&mut self) -> &mut NewInternalActorSystem<DynMessage, B, Strat> {
    &mut self.inner
  }
}
