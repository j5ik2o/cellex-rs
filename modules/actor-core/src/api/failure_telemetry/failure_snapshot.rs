use alloc::{string::String, vec::Vec};

use crate::api::{
  actor::{actor_failure::ActorFailure, ActorId, ActorPath},
  failure_telemetry::{build_snapshot_tags, TelemetryTag},
  supervision::failure::{EscalationStage, FailureInfo, FailureMetadata},
};

/// Failure state captured for telemetry purposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureSnapshot {
  actor:       ActorId,
  path:        ActorPath,
  failure:     ActorFailure,
  metadata:    FailureMetadata,
  stage:       EscalationStage,
  description: String,
  tags:        Vec<TelemetryTag>,
}

impl FailureSnapshot {
  /// Captures an immutable snapshot from [`FailureInfo`].
  /// Creates a snapshot from [`FailureInfo`].
  #[must_use]
  pub fn from_failure_info(info: &FailureInfo) -> Self {
    Self {
      actor:       info.actor,
      path:        info.path.clone(),
      failure:     info.failure.clone(),
      metadata:    info.metadata.clone(),
      stage:       info.stage,
      description: info.description().into_owned(),
      tags:        build_snapshot_tags(&info.metadata),
    }
  }

  /// Returns the actor identifier.
  #[must_use]
  pub const fn actor(&self) -> ActorId {
    self.actor
  }

  /// Returns the actor path.
  #[must_use]
  pub const fn path(&self) -> &ActorPath {
    &self.path
  }

  /// Returns the failure payload.
  #[must_use]
  pub const fn failure(&self) -> &ActorFailure {
    &self.failure
  }

  /// Returns the associated metadata.
  #[must_use]
  pub const fn metadata(&self) -> &FailureMetadata {
    &self.metadata
  }

  /// Returns the escalation stage.
  #[must_use]
  pub const fn stage(&self) -> EscalationStage {
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
