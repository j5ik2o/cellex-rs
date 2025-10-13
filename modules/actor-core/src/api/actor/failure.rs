use alloc::string::String;
use core::fmt;

/// Error information returned from actor message handlers.
#[derive(Clone)]
pub struct ActorFailure {
  message: String,
}

impl ActorFailure {
  /// Creates a new failure with the provided message.
  #[must_use]
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }

  /// Returns the failure message.
  #[must_use]
  pub fn message(&self) -> &str {
    &self.message
  }

  /// Creates a failure from the provided error value.
  #[must_use]
  pub fn from_error<E>(error: E) -> Self
  where
    E: fmt::Display + fmt::Debug + Send + 'static, {
    Self {
      message: alloc::format!("{error:?}"),
    }
  }
}

impl fmt::Debug for ActorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ActorFailure").field("message", &self.message).finish()
  }
}

impl fmt::Display for ActorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}
