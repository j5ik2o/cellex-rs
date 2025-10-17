use crate::api::mailbox::mailbox_runtime::MailboxRuntime;
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Shared;

use super::receive_timeout_driver::ReceiveTimeoutDriver;
use super::receive_timeout_factory_shared::ReceiveTimeoutFactoryShared;

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
