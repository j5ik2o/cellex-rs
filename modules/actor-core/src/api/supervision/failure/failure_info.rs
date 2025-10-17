#[cfg(test)]
mod tests;

use alloc::borrow::Cow;

use crate::api::actor::actor_failure::ActorFailure;
use crate::api::actor::actor_failure::BehaviorFailure;
use crate::api::identity::ActorId;
use crate::api::identity::ActorPath;

use super::{EscalationStage, FailureMetadata};

/// Failure information. Holds a simplified form of protoactor-go's Failure message.
#[derive(Clone, Debug)]
pub struct FailureInfo {
  /// ID of the actor where the failure occurred
  pub actor: ActorId,
  /// Path of the actor where the failure occurred
  pub path: ActorPath,
  /// Detailed failure payload
  pub failure: ActorFailure,
  /// Metadata associated with the failure
  pub metadata: FailureMetadata,
  /// Escalation stage
  pub stage: EscalationStage,
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
  pub fn new_with_metadata(actor: ActorId, path: ActorPath, failure: ActorFailure, metadata: FailureMetadata) -> Self {
    Self {
      actor,
      path,
      failure,
      metadata,
      stage: EscalationStage::Initial,
    }
  }

  /// Sets metadata.
  ///
  /// # Arguments
  /// * `metadata` - Metadata to set
  ///
  /// # Returns
  /// `FailureInfo` instance with metadata set
  pub fn with_metadata(mut self, metadata: FailureMetadata) -> Self {
    self.metadata = metadata;
    self
  }

  /// Sets escalation stage.
  ///
  /// # Arguments
  /// * `stage` - Escalation stage to set
  ///
  /// # Returns
  /// `FailureInfo` instance with escalation stage set
  pub fn with_stage(mut self, stage: EscalationStage) -> Self {
    self.stage = stage;
    self
  }

  /// Creates failure information from an error with default metadata (helper for legacy call sites).
  ///
  /// # Arguments
  /// * `actor` - ID of the actor where the failure occurred
  /// * `path` - Path of the actor where the failure occurred
  /// * `failure` - Actor failure reference
  ///
  /// # Returns
  /// New `FailureInfo` instance
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
  pub fn escalate_to_parent(&self) -> Option<Self> {
    let parent_path = self.path.parent()?;
    let parent_actor = parent_path.last().unwrap_or(self.actor);
    Some(Self {
      actor: parent_actor,
      path: parent_path,
      failure: self.failure.clone(),
      metadata: self.metadata.clone(),
      stage: self.stage.escalate(),
    })
  }

  /// Provides a reference to the underlying `BehaviorFailure`.
  #[must_use]
  pub fn behavior_failure(&self) -> &dyn BehaviorFailure {
    self.failure.behavior()
  }

  /// Provides a reference to the wrapped `ActorFailure`.
  #[must_use]
  pub fn actor_failure(&self) -> &ActorFailure {
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
      && self.metadata == other.metadata
      && self.stage == other.stage
  }
}

impl Eq for FailureInfo {}
