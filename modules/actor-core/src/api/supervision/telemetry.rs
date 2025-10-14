#![cfg_attr(not(feature = "std"), allow(unused_imports))]

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

#[cfg(target_has_atomic = "ptr")]
use spin::Once;

use super::failure::{EscalationStage, FailureMetadata};
use crate::{ActorFailure, ActorId, ActorPath, FailureInfo, FailureTelemetryShared, MetricsEvent, MetricsSinkShared};
use cellex_utils_core_rs::sync::SharedBound;

/// `FailureSnapshot` が保持するタグ数の上限。
pub const MAX_FAILURE_SNAPSHOT_TAGS: usize = 8;

/// Telemetry に渡されるタグ（キー／バリュー）ペア。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryTag {
  key: Cow<'static, str>,
  value: Cow<'static, str>,
}

impl TelemetryTag {
  /// 新しいタグを生成する。
  #[must_use]
  pub fn new(key: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>) -> Self {
    Self {
      key: key.into(),
      value: value.into(),
    }
  }

  /// タグのキーを返す。
  #[must_use]
  pub fn key(&self) -> &str {
    self.key.as_ref()
  }

  /// タグの値を返す。
  #[must_use]
  pub fn value(&self) -> &str {
    self.value.as_ref()
  }
}

/// Telemetry 呼び出しの観測設定。
#[derive(Clone, Default, Debug)]
pub struct TelemetryObservationConfig {
  metrics: Option<MetricsSinkShared>,
  record_timing: bool,
}

impl TelemetryObservationConfig {
  /// 新しい設定を生成する。
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// メトリクスシンクを設定する。
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics = sink;
  }

  /// メトリクスシンクを持つ設定を返す（builder 用）。
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: MetricsSinkShared) -> Self {
    self.metrics = Some(sink);
    self
  }

  /// 現在のメトリクスシンクを参照する。
  #[must_use]
  pub fn metrics_sink(&self) -> Option<&MetricsSinkShared> {
    self.metrics.as_ref()
  }

  /// 呼び出し時間計測の有効／無効を設定する。
  pub fn set_record_timing(&mut self, enabled: bool) {
    self.record_timing = enabled;
  }

  /// 呼び出し時間を記録するかどうか。
  #[must_use]
  pub fn should_record_timing(&self) -> bool {
    self.record_timing
  }

  /// Telemetry 呼び出し後に観測結果を記録する。
  pub fn observe(&self, elapsed: Option<core::time::Duration>) {
    if let Some(metrics) = &self.metrics {
      metrics.with_ref(|sink| {
        sink.record(MetricsEvent::TelemetryInvoked);
        if let Some(duration) = elapsed {
          let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
          sink.record(MetricsEvent::TelemetryLatencyNanos(nanos));
        }
      });
    }
  }
}

/// Failure state captured for telemetry purposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureSnapshot {
  actor: ActorId,
  path: ActorPath,
  failure: ActorFailure,
  metadata: FailureMetadata,
  stage: EscalationStage,
  description: String,
  tags: Vec<TelemetryTag>,
}

impl FailureSnapshot {
  /// Captures an immutable snapshot from [`FailureInfo`].
  /// Creates a snapshot from [`FailureInfo`].
  pub fn from_failure_info(info: &FailureInfo) -> Self {
    Self {
      actor: info.actor,
      path: info.path.clone(),
      failure: info.failure.clone(),
      metadata: info.metadata.clone(),
      stage: info.stage,
      description: info.description().into_owned(),
      tags: build_snapshot_tags(&info.metadata),
    }
  }

  /// Returns the actor identifier.
  #[must_use]
  pub fn actor(&self) -> ActorId {
    self.actor
  }

  /// Returns the actor path.
  #[must_use]
  pub fn path(&self) -> &ActorPath {
    &self.path
  }

  /// Returns the failure payload.
  #[must_use]
  pub fn failure(&self) -> &ActorFailure {
    &self.failure
  }

  /// Returns the associated metadata.
  #[must_use]
  pub fn metadata(&self) -> &FailureMetadata {
    &self.metadata
  }

  /// Returns the escalation stage.
  #[must_use]
  pub fn stage(&self) -> EscalationStage {
    self.stage
  }

  /// Textual description of the failure.
  #[must_use]
  pub fn description(&self) -> &str {
    &self.description
  }

  /// Returns telemetry tags attached to the snapshot.
  #[must_use]
  pub fn tags(&self) -> &[TelemetryTag] {
    &self.tags
  }
}

/// Telemetry hook invoked whenever a failure reaches the root escalation sink.
pub trait FailureTelemetry: SharedBound {
  /// Called with the failure information before any event handlers/listeners run.
  fn on_failure(&self, snapshot: &FailureSnapshot);
}

/// Telemetry implementation that performs no side effects.
#[derive(Default, Clone, Copy)]
pub struct NoopFailureTelemetry;

impl FailureTelemetry for NoopFailureTelemetry {
  fn on_failure(&self, _snapshot: &FailureSnapshot) {}
}

/// Returns a shared handle to the no-op telemetry implementation.
pub fn noop_failure_telemetry() -> FailureTelemetryShared {
  #[cfg(target_has_atomic = "ptr")]
  {
    static INSTANCE: Once<FailureTelemetryShared> = Once::new();
    INSTANCE
      .call_once(|| FailureTelemetryShared::new(NoopFailureTelemetry))
      .clone()
  }

  #[cfg(not(target_has_atomic = "ptr"))]
  {
    FailureTelemetryShared::new(NoopFailureTelemetry)
  }
}

/// Returns the default telemetry implementation for the current build configuration.
pub fn default_failure_telemetry() -> FailureTelemetryShared {
  #[cfg(all(feature = "std", feature = "unwind-supervision"))]
  {
    return tracing_failure_telemetry();
  }

  #[cfg(not(all(feature = "std", feature = "unwind-supervision")))]
  {
    return noop_failure_telemetry();
  }
}

fn build_snapshot_tags(metadata: &FailureMetadata) -> Vec<TelemetryTag> {
  let mut tags = Vec::new();

  if let Some(component) = metadata.component.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(TelemetryTag::new(
        Cow::Borrowed("component"),
        Cow::Owned(component.clone()),
      ));
    }
  }
  if let Some(endpoint) = metadata.endpoint.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(TelemetryTag::new(
        Cow::Borrowed("endpoint"),
        Cow::Owned(endpoint.clone()),
      ));
    }
  }
  if let Some(transport) = metadata.transport.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(TelemetryTag::new(
        Cow::Borrowed("transport"),
        Cow::Owned(transport.clone()),
      ));
    }
  }

  for (key, value) in metadata.tags.iter() {
    if tags.len() >= MAX_FAILURE_SNAPSHOT_TAGS {
      break;
    }
    tags.push(TelemetryTag::new(Cow::Owned(key.clone()), Cow::Owned(value.clone())));
  }

  tags
}

#[cfg(feature = "std")]
/// Telemetry implementation that emits tracing events.
pub struct TracingFailureTelemetry;

#[cfg(feature = "std")]
impl FailureTelemetry for TracingFailureTelemetry {
  fn on_failure(&self, snapshot: &FailureSnapshot) {
    tracing::error!(
      actor = ?snapshot.actor(),
      reason = %snapshot.description(),
      path = ?snapshot.path().segments(),
      stage = ?snapshot.stage(),
      "actor escalation reached root guardian"
    );
  }
}

#[cfg(feature = "std")]
/// Returns a shared handle to the tracing-based telemetry implementation.
pub fn tracing_failure_telemetry() -> FailureTelemetryShared {
  FailureTelemetryShared::new(TracingFailureTelemetry)
}

#[cfg(test)]
mod tests;
