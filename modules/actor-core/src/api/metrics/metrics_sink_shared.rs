use cellex_utils_core_rs::sync::{ArcShared, Shared, SharedBound};
use core::fmt;

use super::metrics_sink::MetricsSink;

/// `MetricsSink` を共有するためのラッパー。
pub struct MetricsSinkShared {
  inner: ArcShared<dyn MetricsSink>,
}

impl MetricsSinkShared {
  /// 具体的なシンク実装から共有ラッパーを生成する。
  #[must_use]
  pub fn new<S>(sink: S) -> Self
  where
    S: MetricsSink + SharedBound + 'static, {
    let shared = ArcShared::new(sink);
    Self {
      inner: shared.into_dyn(|inner| inner as &dyn MetricsSink),
    }
  }

  /// 既存の共有シンクをラップする。
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn MetricsSink>) -> Self {
    Self { inner }
  }

  /// 内部に保持している共有ハンドルを取り出す。
  #[must_use]
  pub fn into_shared(self) -> ArcShared<dyn MetricsSink> {
    self.inner
  }

  /// 共有ハンドルへの参照を利用してクロージャを実行する。
  pub fn with_ref<R>(&self, f: impl FnOnce(&dyn MetricsSink) -> R) -> R {
    self.inner.with_ref(f)
  }
}

impl Clone for MetricsSinkShared {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl fmt::Debug for MetricsSinkShared {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("MetricsSinkShared(..)")
  }
}
