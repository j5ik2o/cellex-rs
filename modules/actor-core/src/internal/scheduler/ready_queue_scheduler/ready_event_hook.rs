// allow:multi-types
use cellex_utils_core_rs::sync::ArcShared;

/// Hook invoked by mailboxes when new messages arrive.
#[cfg(target_has_atomic = "ptr")]
pub trait ReadyEventHook: Send + Sync {
  /// Notifies the scheduler that the associated actor has become ready.
  fn notify_ready(&self);
}

/// Hook invoked by mailboxes when new messages arrive (no atomic pointer targets).
#[cfg(not(target_has_atomic = "ptr"))]
pub trait ReadyEventHook {
  /// Notifies the scheduler that the associated actor has become ready.
  fn notify_ready(&self);
}

/// Shared handle to a [`ReadyEventHook`].
#[cfg(target_has_atomic = "ptr")]
pub type ReadyQueueHandle = ArcShared<dyn ReadyEventHook + Send + Sync>;

/// Shared handle to a [`ReadyEventHook`] (no atomic pointer targets).
#[cfg(not(target_has_atomic = "ptr"))]
pub type ReadyQueueHandle = ArcShared<dyn ReadyEventHook>;
