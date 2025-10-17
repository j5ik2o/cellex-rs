use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Shared;

use super::receive_timeout_scheduler_factory_provider::ReceiveTimeoutSchedulerFactoryProvider;
use super::receive_timeout_scheduler_factory_shared::ReceiveTimeoutSchedulerFactoryShared;

/// Shared wrapper around a [`ReceiveTimeoutSchedulerFactoryProvider`] implementation.
pub struct ReceiveTimeoutSchedulerFactoryProviderShared<R> {
  inner: ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<R>>,
}

impl<R> ReceiveTimeoutSchedulerFactoryProviderShared<R>
where
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  /// Creates a new shared driver from a concrete driver value.
  #[must_use]
  pub fn new<D>(driver: D) -> Self
  where
    D: ReceiveTimeoutSchedulerFactoryProvider<R> + 'static, {
    let shared = ArcShared::new(driver);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn ReceiveTimeoutSchedulerFactoryProvider<R>),
    }
  }

  /// Wraps an existing shared driver.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<R>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<R>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<R>> {
    &self.inner
  }

  /// Builds a factory by delegating to the underlying driver.
  #[must_use]
  pub fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<DynMessage, R> {
    self.inner.with_ref(|driver| driver.build_factory())
  }
}

impl<R> Clone for ReceiveTimeoutSchedulerFactoryProviderShared<R> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<R> core::ops::Deref for ReceiveTimeoutSchedulerFactoryProviderShared<R> {
  type Target = dyn ReceiveTimeoutSchedulerFactoryProvider<R>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
