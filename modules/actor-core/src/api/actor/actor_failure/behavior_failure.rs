use alloc::borrow::Cow;
use alloc::format;
use core::any::Any;
use core::fmt;

/// Abstraction exposed to supervisors when a behavior fails.
pub trait BehaviorFailure: fmt::Debug + Send + Sync + 'static {
  /// Allows downcasting to the concrete failure type.
  fn as_any(&self) -> &dyn Any;

  /// Human-readable description, intended for logs.
  fn description(&self) -> Cow<'_, str> {
    Cow::Owned(format!("{:?}", self))
  }
}
