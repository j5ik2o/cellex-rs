#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;

use cellex_utils_core_rs::sync::{ArcShared, Shared, SharedBound};
use core::fmt;

/// メトリクスイベントを表す種別。
///
/// 現状は概要レベルの区分のみを提供し、詳細なペイロードは後続フェーズで拡張する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsEvent {
  /// アクターがスケジューラに登録された。
  ActorRegistered,
  /// アクターが停止し、スケジューラから削除された。
  ActorDeregistered,
  /// メールボックスへユーザーメッセージがキューイングされた。
  MailboxEnqueued,
  /// メールボックスからメッセージがデキューされた。
  MailboxDequeued,
  /// テレメトリ呼び出しが実行された。
  TelemetryInvoked,
  /// テレメトリ呼び出しに要した時間（ナノ秒）。
  TelemetryLatencyNanos(u64),
}

/// ランタイムがメトリクスを発行するための抽象シンク。
pub trait MetricsSink: Send + Sync + 'static {
  /// メトリクスイベントを記録する。
  fn record(&self, event: MetricsEvent);
}

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
    Self {
      inner: ArcShared::from_arc(Arc::new(sink) as Arc<dyn MetricsSink>),
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

/// 何も記録しないノップ実装。
#[derive(Clone, Default)]
pub struct NoopMetricsSink;

impl MetricsSink for NoopMetricsSink {
  fn record(&self, _event: MetricsEvent) {
    // intentionally noop
  }
}
