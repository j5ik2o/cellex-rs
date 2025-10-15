use alloc::boxed::Box;
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use core::ops::Deref;

use crate::api::actor::RuntimeEnv;
use crate::runtime::message::DynMessage;
use crate::runtime::metrics::MetricsSinkShared;
use crate::runtime::scheduler::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory};
use crate::Extensions;
use crate::{FailureEvent, FailureInfo, FailureTelemetry, MailboxRuntime, PriorityEnvelope, SystemMessage};
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use cellex_utils_core_rs::Element;
use cellex_utils_core_rs::Shared;

#[cfg(target_has_atomic = "ptr")]
type MapSystemFn<M> = dyn Fn(SystemMessage) -> M + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type MapSystemFn<M> = dyn Fn(SystemMessage) -> M;

#[cfg(target_has_atomic = "ptr")]
type FailureEventHandlerFn = dyn Fn(&FailureInfo) + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type FailureEventHandlerFn = dyn Fn(&FailureInfo);

#[cfg(target_has_atomic = "ptr")]
type FailureEventListenerFn = dyn Fn(FailureEvent) + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type FailureEventListenerFn = dyn Fn(FailureEvent);

#[cfg(target_has_atomic = "ptr")]
pub trait ReceiveTimeoutDriverBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutDriverBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait ReceiveTimeoutDriverBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutDriverBound for T {}

/// Shared handle to a system message mapper function.
///
/// Internally stores the mapper inside a `Shared` abstraction so that
/// different backends (`Arc`, `Rc`, etc.) can be plugged in later without
/// touching the call sites in `actor-core`.
pub struct MapSystemShared<M> {
  inner: ArcShared<MapSystemFn<M>>,
}

impl<M> MapSystemShared<M> {
  /// Creates a new shared mapper from a function or closure.
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(SystemMessage) -> M + SharedBound + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(f)),
    }
  }

  /// Wraps an existing shared mapper.
  pub fn from_shared(inner: ArcShared<MapSystemFn<M>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying `Arc`.
  pub fn into_arc(self) -> Arc<MapSystemFn<M>> {
    self.inner.into_arc_for_testing_dont_use_production()
  }

  /// Returns the inner shared handle.
  pub fn as_shared(&self) -> &ArcShared<MapSystemFn<M>> {
    &self.inner
  }
}

impl<M> Clone for MapSystemShared<M> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<M> Deref for MapSystemShared<M> {
  type Target = MapSystemFn<M>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Shared wrapper around a `ReceiveTimeoutSchedulerFactory` implementation.
pub struct ReceiveTimeoutFactoryShared<M, R> {
  inner: ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>>,
}

impl<M, R> ReceiveTimeoutFactoryShared<M, R>
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  /// Creates a new shared factory from a concrete factory value.
  pub fn new<F>(factory: F) -> Self
  where
    F: ReceiveTimeoutSchedulerFactory<M, R> + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(factory)),
    }
  }

  /// Wraps an existing shared factory.
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>> {
    self.inner
  }

  /// Adapts the factory to operate with [`RuntimeEnv`] as the runtime type.
  pub fn for_runtime_bundle(&self) -> ReceiveTimeoutFactoryShared<M, RuntimeEnv<R>>
  where
    R: MailboxRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<M>>: Clone,
    R::Signal: Clone,
    R::Producer<PriorityEnvelope<M>>: Clone, {
    ReceiveTimeoutFactoryShared::from_shared(ArcShared::from_arc_for_testing_dont_use_production(Arc::new(ReceiveTimeoutFactoryAdapter {
      inner: self.inner.clone(),
    })))
  }
}

impl<M, R> Clone for ReceiveTimeoutFactoryShared<M, R> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<M, R> Deref for ReceiveTimeoutFactoryShared<M, R> {
  type Target = dyn ReceiveTimeoutSchedulerFactory<M, R>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

struct ReceiveTimeoutFactoryAdapter<M, R>
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone, {
  inner: ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>>,
}

impl<M, R> ReceiveTimeoutSchedulerFactory<M, RuntimeEnv<R>> for ReceiveTimeoutFactoryAdapter<M, R>
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  fn create(
    &self,
    sender: <RuntimeEnv<R> as MailboxRuntime>::Producer<PriorityEnvelope<M>>,
    map_system: MapSystemShared<M>,
  ) -> Box<dyn ReceiveTimeoutScheduler> {
    self.inner.create(sender, map_system)
  }
}

/// Trait representing a runtime-specific provider for receive-timeout scheduler factories.
pub trait ReceiveTimeoutDriver<R>: ReceiveTimeoutDriverBound
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone, {
  /// Builds a shared factory bound to [`RuntimeEnv`] for the given runtime.
  fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, RuntimeEnv<R>>;
}

/// Shared wrapper around a [`ReceiveTimeoutDriver`] implementation.
pub struct ReceiveTimeoutDriverShared<R> {
  inner: ArcShared<dyn ReceiveTimeoutDriver<R>>,
}

impl<R> ReceiveTimeoutDriverShared<R>
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  /// Creates a new shared driver from a concrete driver value.
  pub fn new<D>(driver: D) -> Self
  where
    D: ReceiveTimeoutDriver<R> + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(driver)),
    }
  }

  /// Wraps an existing shared driver.
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutDriver<R>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutDriver<R>> {
    self.inner
  }

  /// Builds a factory by delegating to the underlying driver.
  pub fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, RuntimeEnv<R>> {
    self.inner.with_ref(|driver| driver.build_factory())
  }

  /// Returns the inner shared handle.
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutDriver<R>> {
    &self.inner
  }
}

impl<R> Clone for ReceiveTimeoutDriverShared<R> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<R> Deref for ReceiveTimeoutDriverShared<R> {
  type Target = dyn ReceiveTimeoutDriver<R>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Shared wrapper around a [`FailureTelemetry`] implementation.
pub struct FailureTelemetryShared {
  inner: ArcShared<dyn FailureTelemetry>,
}

impl FailureTelemetryShared {
  /// Creates a new shared telemetry handle from a concrete implementation.
  #[must_use]
  pub fn new<T>(telemetry: T) -> Self
  where
    T: FailureTelemetry + SharedBound + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(telemetry) as Arc<dyn FailureTelemetry>),
    }
  }

  /// Wraps an existing shared telemetry handle.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn FailureTelemetry>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn FailureTelemetry> {
    self.inner
  }

  /// Executes the provided closure with a shared reference to the telemetry implementation.
  pub fn with_ref<R>(&self, f: impl FnOnce(&dyn FailureTelemetry) -> R) -> R {
    self.inner.with_ref(|inner| f(inner))
  }
}

impl Clone for FailureTelemetryShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl core::ops::Deref for FailureTelemetryShared {
  type Target = dyn FailureTelemetry;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Context provided to telemetry builders.
pub struct TelemetryContext {
  metrics: Option<MetricsSinkShared>,
  extensions: Extensions,
}

impl TelemetryContext {
  /// Creates a new telemetry context with optional metrics sink information.
  #[must_use]
  pub fn new(metrics: Option<MetricsSinkShared>, extensions: Extensions) -> Self {
    Self { metrics, extensions }
  }

  /// Returns the metrics sink associated with the context, if any.
  #[must_use]
  pub fn metrics_sink(&self) -> Option<&MetricsSinkShared> {
    self.metrics.as_ref()
  }

  /// Returns the extension registry reference.
  #[must_use]
  pub fn extensions(&self) -> &Extensions {
    &self.extensions
  }
}

trait TelemetryBuilderFn: SharedBound {
  fn build(&self, ctx: &TelemetryContext) -> FailureTelemetryShared;
}

impl<F> TelemetryBuilderFn for F
where
  F: Fn(&TelemetryContext) -> FailureTelemetryShared + SharedBound,
{
  fn build(&self, ctx: &TelemetryContext) -> FailureTelemetryShared {
    (self)(ctx)
  }
}

/// Shared wrapper around a failure telemetry builder function.
pub struct FailureTelemetryBuilderShared {
  inner: ArcShared<dyn TelemetryBuilderFn>,
}

impl FailureTelemetryBuilderShared {
  /// Creates a new shared telemetry builder from the provided closure.
  #[must_use]
  pub fn new<F>(builder: F) -> Self
  where
    F: Fn(&TelemetryContext) -> FailureTelemetryShared + SharedBound + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(builder)),
    }
  }

  /// Executes the builder to obtain a telemetry implementation.
  #[must_use]
  pub fn build(&self, ctx: &TelemetryContext) -> FailureTelemetryShared {
    self.inner.with_ref(|builder| builder.build(ctx))
  }
}

impl Clone for FailureTelemetryBuilderShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::NoopFailureTelemetry;

  #[test]
  fn telemetry_builder_shared_invokes_closure() {
    let extensions = Extensions::new();
    let builder = FailureTelemetryBuilderShared::new(|_ctx| FailureTelemetryShared::new(NoopFailureTelemetry));
    let ctx = TelemetryContext::new(None, extensions.clone());

    let telemetry = builder.build(&ctx);
    telemetry.with_ref(|_impl| {});
  }
}

/// Shared wrapper for failure event handlers.
pub struct FailureEventHandlerShared {
  inner: ArcShared<FailureEventHandlerFn>,
}

impl FailureEventHandlerShared {
  /// Creates a new shared handler from a closure.
  pub fn new<F>(handler: F) -> Self
  where
    F: Fn(&FailureInfo) + SharedBound + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(handler)),
    }
  }

  /// Wraps an existing shared handler reference.
  pub fn from_shared(inner: ArcShared<FailureEventHandlerFn>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handler.
  pub fn into_shared(self) -> ArcShared<FailureEventHandlerFn> {
    self.inner
  }
}

impl Clone for FailureEventHandlerShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl Deref for FailureEventHandlerShared {
  type Target = FailureEventHandlerFn;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Shared wrapper for failure event listeners.
pub struct FailureEventListenerShared {
  inner: ArcShared<FailureEventListenerFn>,
}

impl FailureEventListenerShared {
  /// Creates a new shared listener from a closure.
  pub fn new<F>(listener: F) -> Self
  where
    F: Fn(FailureEvent) + SharedBound + 'static, {
    Self {
      inner: ArcShared::from_arc_for_testing_dont_use_production(Arc::new(listener)),
    }
  }

  /// Wraps an existing shared listener.
  pub fn from_shared(inner: ArcShared<FailureEventListenerFn>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared listener.
  pub fn into_shared(self) -> ArcShared<FailureEventListenerFn> {
    self.inner
  }
}

impl Clone for FailureEventListenerShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl Deref for FailureEventListenerShared {
  type Target = FailureEventListenerFn;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
