use alloc::{borrow::Cow, format};
use core::{any::Any, fmt};

use cellex_utils_core_rs::sync::shared::SharedBound;

/// Abstraction exposed to supervisors when a behavior fails.
pub trait BehaviorFailure: fmt::Debug + SharedBound + 'static {
  /// Allows downcasting to the concrete failure type.
  fn as_any(&self) -> &dyn Any;

  /// Human-readable description, intended for logs.
  fn description(&self) -> Cow<'_, str> {
    Cow::Owned(format!("{:?}", self))
  }
}
