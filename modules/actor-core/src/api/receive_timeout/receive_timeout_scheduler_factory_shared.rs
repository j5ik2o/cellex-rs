use cellex_utils_core_rs::{sync::ArcShared, Element};

use crate::api::{
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  receive_timeout::ReceiveTimeoutSchedulerFactory,
};

/// Shared wrapper around a `ReceiveTimeoutSchedulerFactory` implementation.
pub struct ReceiveTimeoutSchedulerFactoryShared<M, MF> {
  inner: ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, MF>>,
}

impl<M, MF> ReceiveTimeoutSchedulerFactoryShared<M, MF>
where
  M: Element + 'static,
  MF: MailboxFactory + Clone + 'static,
  MF::Producer<PriorityEnvelope<M>>: Clone,
{
  /// Creates a new shared factory from a concrete factory value.
  #[must_use]
  pub fn new<F>(factory: F) -> Self
  where
    F: ReceiveTimeoutSchedulerFactory<M, MF> + 'static, {
    let shared = ArcShared::new(factory);
    Self { inner: shared.into_dyn(|inner| inner as &dyn ReceiveTimeoutSchedulerFactory<M, MF>) }
  }

  /// Wraps an existing shared factory.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, MF>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, MF>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<dyn ReceiveTimeoutSchedulerFactory<M, MF>> {
    &self.inner
  }
}

impl<M, MF> Clone for ReceiveTimeoutSchedulerFactoryShared<M, MF> {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl<M, MF> core::ops::Deref for ReceiveTimeoutSchedulerFactoryShared<M, MF> {
  type Target = dyn ReceiveTimeoutSchedulerFactory<M, MF>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
