use alloc::{borrow::Cow, format, string::String};
use core::{any::Any, fmt};

use super::behavior_failure::BehaviorFailure;

/// Default implementation of [`BehaviorFailure`].
#[derive(Clone, Debug)]
pub struct DefaultBehaviorFailure {
  message: Cow<'static, str>,
  debug:   Option<String>,
}

impl DefaultBehaviorFailure {
  /// Creates a failure representation from a message.
  #[must_use]
  pub fn from_message(message: impl Into<Cow<'static, str>>) -> Self {
    Self { message: message.into(), debug: None }
  }

  /// Creates a failure representation from an error type implementing [`fmt::Display`].
  #[must_use]
  pub fn from_error<E>(error: E) -> Self
  where
    E: fmt::Display + fmt::Debug, {
    Self { message: Cow::Owned(format!("{error}")), debug: Some(format!("{error:?}")) }
  }

  /// Fallback used when the panic payload type is unknown.
  #[must_use]
  pub fn from_unknown_panic(payload: &(dyn Any + Send)) -> Self {
    Self {
      message: Cow::Owned(String::from("panic: unknown payload")),
      debug:   Some(format!("panic payload type_id: {:?}", payload.type_id())),
    }
  }

  /// Returns optional debug details if available.
  #[must_use]
  pub fn debug_details(&self) -> Option<&str> {
    self.debug.as_deref()
  }
}

impl BehaviorFailure for DefaultBehaviorFailure {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn description(&self) -> Cow<'_, str> {
    self.message.clone()
  }
}

impl fmt::Display for DefaultBehaviorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.message.as_ref())
  }
}
