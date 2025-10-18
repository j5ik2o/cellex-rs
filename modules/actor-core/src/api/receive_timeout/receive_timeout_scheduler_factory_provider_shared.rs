use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Shared;

use super::receive_timeout_scheduler_factory_provider::ReceiveTimeoutSchedulerFactoryProvider;
use super::receive_timeout_scheduler_factory_shared::ReceiveTimeoutSchedulerFactoryShared;

/// Shared wrapper around a [`ReceiveTimeoutSchedulerFactoryProvider`] implementation.
pub struct ReceiveTimeoutSchedulerFactoryProviderShared<MF> {
  inner: ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<MF>>,
}

impl<MF> ReceiveTimeoutSchedulerFactoryProviderShared<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<DynMessage>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  /// Creates a new shared driver from a concrete driver value.
  #[must_use]
  pub fn new<D>(driver: D) -> Self
  where
    D: ReceiveTimeoutSchedulerFactoryProvider<MF> + 'static, {
    let shared = ArcShared::new(driver);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn ReceiveTimeoutSchedulerFactoryProvider<MF>),
    }
  }

  /// Wraps an existing shared driver.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<MF>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<MF>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutSchedulerFactoryProvider<MF>> {
    &self.inner
  }

  /// Builds a factory by delegating to the underlying driver.
  #[must_use]
  pub fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<DynMessage, MF> {
    self.inner.with_ref(|driver| driver.build_factory())
  }
}

impl<MF> Clone for ReceiveTimeoutSchedulerFactoryProviderShared<MF> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<MF> core::ops::Deref for ReceiveTimeoutSchedulerFactoryProviderShared<MF> {
  type Target = dyn ReceiveTimeoutSchedulerFactoryProvider<MF>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
