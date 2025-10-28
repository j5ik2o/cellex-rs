// allow:multi-types
use cellex_utils_core_rs::sync::{shared::SharedBound, ArcShared};

/// Hook invoked by mailboxes when new messages arrive.
pub trait ReadyEventHook: SharedBound {
  /// Notifies the scheduler that the associated actor has become ready.
  fn notify_ready(&self);
}

/// Shared handle to a ready-event hook implementation.
pub type ReadyQueueHandle = ArcShared<dyn ReadyEventHook>;
