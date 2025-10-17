use crate::api::mailbox::PriorityEnvelope;
use crate::internal::scheduler::ReceiveTimeoutSchedulerFactory;
use crate::{DynMessage, MailboxRuntime};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, Shared};

#[cfg(target_has_atomic = "ptr")]
pub trait ReceiveTimeoutDriverBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutDriverBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait ReceiveTimeoutDriverBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutDriverBound for T {}

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
  #[must_use]
  pub fn new<F>(factory: F) -> Self
  where
    F: ReceiveTimeoutSchedulerFactory<M, R> + 'static, {
    let shared = ArcShared::new(factory);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn ReceiveTimeoutSchedulerFactory<M, R>),
    }
  }

  /// Wraps an existing shared factory.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, R>> {
    &self.inner
  }
}

impl<M, R> Clone for ReceiveTimeoutFactoryShared<M, R> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<M, R> core::ops::Deref for ReceiveTimeoutFactoryShared<M, R> {
  type Target = dyn ReceiveTimeoutSchedulerFactory<M, R>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

/// Trait representing a runtime-specific provider for receive-timeout scheduler factories.
pub trait ReceiveTimeoutDriver<R>: ReceiveTimeoutDriverBound
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone, {
  /// Builds a shared factory bound to the mailbox runtime for the given actor runtime.
  fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, R>;
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
  #[must_use]
  pub fn new<D>(driver: D) -> Self
  where
    D: ReceiveTimeoutDriver<R> + 'static, {
    let shared = ArcShared::new(driver);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn ReceiveTimeoutDriver<R>),
    }
  }

  /// Wraps an existing shared driver.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutDriver<R>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutDriver<R>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutDriver<R>> {
    &self.inner
  }

  /// Builds a factory by delegating to the underlying driver.
  #[must_use]
  pub fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, R> {
    self.inner.with_ref(|driver| driver.build_factory())
  }
}

impl<R> Clone for ReceiveTimeoutDriverShared<R> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<R> core::ops::Deref for ReceiveTimeoutDriverShared<R> {
  type Target = dyn ReceiveTimeoutDriver<R>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
