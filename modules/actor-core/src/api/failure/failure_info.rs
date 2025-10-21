#[cfg(test)]
mod tests;

use alloc::borrow::Cow;

use super::{metadata::FailureEscalationStage, FailureMetadata};
use crate::api::actor::{
  actor_failure::{ActorFailure, BehaviorFailure},
  ActorId, ActorPath,
};

/// Failure information. Holds a simplified form of protoactor-go's Failure message.
#[derive(Clone, Debug)]
pub struct FailureInfo {
  /// ID of the actor where the failure occurred
  pub actor:                    ActorId,
  /// Path of the actor where the failure occurred
  pub path:                     ActorPath,
  /// Detailed failure payload
  pub failure:                  ActorFailure,
  /// Metadata associated with the failure
  pub failure_metadata:         FailureMetadata,
  /// Escalation stage
  pub failure_escalation_stage: FailureEscalationStage,
}

impl FailureInfo {
  /// Creates new failure information with default metadata.
  ///
  /// # Arguments
  /// * `actor` - ID of the actor where the failure occurred
  /// * `path` - Path of the actor where the failure occurred
  /// * `failure` - Failure payload
  ///
  /// # Returns
  /// New `FailureInfo` instance
  #[must_use]
  pub fn new(actor: ActorId, path: ActorPath, failure: ActorFailure) -> Self {
    Self::new_with_metadata(actor, path, failure, FailureMetadata::default())
  }

  /// Creates new failure information with specified metadata.
  ///
  /// # Arguments
  /// * `actor` - ID of the actor where the failure occurred
  /// * `path` - Path of the actor where the failure occurred
  /// * `failure` - Failure payload
  /// * `metadata` - Metadata associated with the failure
  ///
  /// # Returns
  /// New `FailureInfo` instance
  #[must_use]
  pub const fn new_with_metadata(
    actor: ActorId,
    path: ActorPath,
    failure: ActorFailure,
    metadata: FailureMetadata,
  ) -> Self {
    Self { actor, path, failure, failure_metadata: metadata, failure_escalation_stage: FailureEscalationStage::Initial }
  }

  /// Sets metadata.
  ///
  /// # Arguments
  /// * `metadata` - Metadata to set
  ///
  /// # Returns
  /// `FailureInfo` instance with metadata set
  #[must_use]
  pub fn with_metadata(mut self, metadata: FailureMetadata) -> Self {
    self.failure_metadata = metadata;
    self
  }

  /// Sets escalation stage.
  ///
  /// # Arguments
  /// * `stage` - Escalation stage to set
  ///
  /// # Returns
  /// `FailureInfo` instance with escalation stage set
  #[must_use]
  pub const fn with_stage(mut self, stage: FailureEscalationStage) -> Self {
    self.failure_escalation_stage = stage;
    self
  }

  /// Creates failure information from an error with default metadata (helper for legacy call
  /// sites).
  ///
  /// # Arguments
  /// * `actor` - ID of the actor where the failure occurred
  /// * `path` - Path of the actor where the failure occurred
  /// * `failure` - Actor failure reference
  ///
  /// # Returns
  /// New `FailureInfo` instance
  #[must_use]
  pub fn from_error(actor: ActorId, path: ActorPath, failure: &ActorFailure) -> Self {
    Self::from_error_with_metadata(actor, path, failure, FailureMetadata::default())
  }

  /// Creates failure information from an error and metadata (helper for legacy call sites).
  ///
  /// # Arguments
  /// * `actor` - ID of the actor where the failure occurred
  /// * `path` - Path of the actor where the failure occurred
  /// * `failure` - Actor failure reference
  /// * `metadata` - Metadata associated with the failure
  ///
  /// # Returns
  /// New `FailureInfo` instance
  #[must_use]
  pub fn from_error_with_metadata(
    actor: ActorId,
    path: ActorPath,
    failure: &ActorFailure,
    metadata: FailureMetadata,
  ) -> Self {
    Self::new_with_metadata(actor, path, failure.clone(), metadata)
  }

  /// Creates failure information from an `ActorFailure`.
  #[must_use]
  pub fn from_failure(actor: ActorId, path: ActorPath, failure: ActorFailure) -> Self {
    Self::new(actor, path, failure)
  }

  /// Creates new failure information escalated to parent actor.
  ///
  /// # Returns
  /// `FailureInfo` instance escalated to parent actor.
  /// Returns `None` if parent doesn't exist.
  #[must_use]
  pub fn escalate_to_parent(&self) -> Option<Self> {
    let parent_path = self.path.parent()?;
    let parent_actor = parent_path.last().unwrap_or(self.actor);
    Some(Self {
      actor:                    parent_actor,
      path:                     parent_path,
      failure:                  self.failure.clone(),
      failure_metadata:         self.failure_metadata.clone(),
      failure_escalation_stage: self.failure_escalation_stage.escalate(),
    })
  }

  /// Provides a reference to the underlying `BehaviorFailure`.
  #[must_use]
  pub fn behavior_failure(&self) -> &dyn BehaviorFailure {
    self.failure.behavior()
  }

  /// Provides a reference to the wrapped `ActorFailure`.
  #[must_use]
  pub const fn actor_failure(&self) -> &ActorFailure {
    &self.failure
  }

  /// Returns a textual description suitable for logging.
  #[must_use]
  pub fn description(&self) -> Cow<'_, str> {
    self.failure.description()
  }
}

impl PartialEq for FailureInfo {
  fn eq(&self, other: &Self) -> bool {
    self.actor == other.actor
      && self.path == other.path
      && self.failure == other.failure
      && self.failure_metadata == other.failure_metadata
      && self.failure_escalation_stage == other.failure_escalation_stage
  }
}

impl Eq for FailureInfo {}
