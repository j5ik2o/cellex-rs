use crate::FailureInfo;
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};

#[cfg(target_has_atomic = "ptr")]
type FailureEventHandlerFn = dyn Fn(&FailureInfo) + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type FailureEventHandlerFn = dyn Fn(&FailureInfo);

/// Shared wrapper for failure event handlers.
pub struct FailureEventHandlerShared {
  inner: ArcShared<FailureEventHandlerFn>,
}

impl FailureEventHandlerShared {
  /// Creates a new shared handler from a closure.
  #[must_use]
  pub fn new<F>(handler: F) -> Self
  where
    F: Fn(&FailureInfo) + SharedBound + 'static, {
    let shared = ArcShared::new(handler);
    Self {
      inner: shared.into_dyn(|inner| inner as &FailureEventHandlerFn),
    }
  }

  /// Wraps an existing shared handler reference.
  #[must_use]
  pub fn from_shared(inner: ArcShared<FailureEventHandlerFn>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handler.
  #[must_use]
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

impl core::ops::Deref for FailureEventHandlerShared {
  type Target = FailureEventHandlerFn;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
