use crate::api::mailbox::mailbox_runtime::MailboxRuntime;
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::internal::scheduler::ReceiveTimeoutSchedulerFactory;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

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
