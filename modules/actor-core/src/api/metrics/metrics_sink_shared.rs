use core::fmt;

use cellex_utils_core_rs::sync::{
  shared::{Shared, SharedBound},
  ArcShared,
};

use super::metrics_sink::MetricsSink;

/// Shared wrapper around a [`MetricsSink`].
pub struct MetricsSinkShared {
  inner: ArcShared<dyn MetricsSink>,
}

impl MetricsSinkShared {
  /// Creates a shared wrapper from a concrete sink implementation.
  #[must_use]
  pub fn new<S>(sink: S) -> Self
  where
    S: MetricsSink + SharedBound + 'static, {
    let shared = ArcShared::new(sink);
    Self { inner: shared.into_dyn(|inner| inner as &dyn MetricsSink) }
  }

  /// Wraps an existing shared sink handle.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn MetricsSink>) -> Self {
    Self { inner }
  }

  /// Extracts the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn MetricsSink> {
    self.inner
  }

  /// Executes a closure with a reference to the underlying sink.
  pub fn with_ref<R>(&self, f: impl FnOnce(&dyn MetricsSink) -> R) -> R {
    self.inner.with_ref(f)
  }
}

impl Clone for MetricsSinkShared {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl fmt::Debug for MetricsSinkShared {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("MetricsSinkShared(..)")
  }
}
