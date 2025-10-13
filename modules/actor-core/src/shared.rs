use alloc::boxed::Box;
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use core::ops::Deref;

use crate::api::actor::RuntimeEnv;
use crate::runtime::message::DynMessage;
use crate::runtime::scheduler::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory};
use crate::{MailboxRuntime, FailureEvent, FailureInfo, PriorityEnvelope, SystemMessage};
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
      inner: ArcShared::from_arc(Arc::new(f)),
    }
  }

  /// Wraps an existing shared mapper.
  pub fn from_shared(inner: ArcShared<MapSystemFn<M>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying `Arc`.
  pub fn into_arc(self) -> Arc<MapSystemFn<M>> {
    self.inner.into_arc()
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
      inner: ArcShared::from_arc(Arc::new(factory)),
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
    ReceiveTimeoutFactoryShared::from_shared(ArcShared::from_arc(Arc::new(ReceiveTimeoutFactoryAdapter {
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
pub trait ReceiveTimeoutDriver<R>: Send + Sync
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
      inner: ArcShared::from_arc(Arc::new(driver)),
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
      inner: ArcShared::from_arc(Arc::new(handler)),
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
      inner: ArcShared::from_arc(Arc::new(listener)),
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
