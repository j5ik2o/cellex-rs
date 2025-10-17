use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Shared;

use super::receive_timeout_factory_provider::ReceiveTimeoutFactoryProvider;
use super::receive_timeout_scheduler_factory_shared::ReceiveTimeoutSchedulerFactoryShared;

/// Shared wrapper around a [`ReceiveTimeoutFactoryProvider`] implementation.
pub struct ReceiveTimeoutFactoryProviderShared<R> {
  inner: ArcShared<dyn ReceiveTimeoutFactoryProvider<R>>,
}

impl<R> ReceiveTimeoutFactoryProviderShared<R>
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
    D: ReceiveTimeoutFactoryProvider<R> + 'static, {
    let shared = ArcShared::new(driver);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn ReceiveTimeoutFactoryProvider<R>),
    }
  }

  /// Wraps an existing shared driver.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutFactoryProvider<R>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutFactoryProvider<R>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutFactoryProvider<R>> {
    &self.inner
  }

  /// Builds a factory by delegating to the underlying driver.
  #[must_use]
  pub fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<DynMessage, R> {
    self.inner.with_ref(|driver| driver.build_factory())
  }
}

impl<R> Clone for ReceiveTimeoutFactoryProviderShared<R> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<R> core::ops::Deref for ReceiveTimeoutFactoryProviderShared<R> {
  type Target = dyn ReceiveTimeoutFactoryProvider<R>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
