use cellex_utils_core_rs::sync::SharedBound;
use core::any::Any;
use portable_atomic::{AtomicI32, Ordering};

/// Identifier type assigned to each [`Extension`].
pub type ExtensionId = i32;

static NEXT_EXTENSION_ID: AtomicI32 = AtomicI32::new(0);

/// Generates a new [`ExtensionId`].
#[must_use]
pub fn next_extension_id() -> ExtensionId {
  NEXT_EXTENSION_ID.fetch_add(1, Ordering::SeqCst)
}

/// Shared interface that user-defined extensions must implement.
pub trait Extension: Any + SharedBound {
  /// Returns the identifier uniquely associated with this extension.
  fn extension_id(&self) -> ExtensionId;

  /// Type-erased accessor used for downcasting.
  fn as_any(&self) -> &dyn Any;
}
