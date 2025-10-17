use super::behavior_failure::BehaviorFailure;
use super::default_behavior_failure::DefaultBehaviorFailure;
use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use cellex_utils_core_rs::sync::ArcShared;
use core::any::Any;
use core::fmt;
use core::ptr;

/// Wrapper passed to supervisors when actor execution fails.
#[derive(Clone)]
pub struct ActorFailure {
  inner: ArcShared<dyn BehaviorFailure>,
}

impl ActorFailure {
  /// Wraps a [`BehaviorFailure`] into an [`ActorFailure`].
  #[must_use]
  pub fn new(inner: impl BehaviorFailure) -> Self {
    let shared = ArcShared::new(inner);
    Self {
      inner: shared.into_dyn(|value| value as &dyn BehaviorFailure),
    }
  }

  /// Creates an [`ActorFailure`] from an existing shared pointer.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn BehaviorFailure>) -> Self {
    Self { inner }
  }

  /// Creates a failure from a message.
  #[must_use]
  pub fn from_message(message: impl Into<Cow<'static, str>>) -> Self {
    Self::new(DefaultBehaviorFailure::from_message(message))
  }

  /// Creates a failure from any error implementing [`fmt::Display`].
  #[must_use]
  pub fn from_error<E>(error: E) -> Self
  where
    E: fmt::Display + fmt::Debug, {
    Self::new(DefaultBehaviorFailure::from_error(error))
  }

  /// Converts a panic payload into a standardized failure.
  #[must_use]
  pub fn from_panic_payload(payload: &(dyn Any + Send)) -> Self {
    if let Some(failure) = payload.downcast_ref::<ActorFailure>() {
      return failure.clone();
    }

    if let Some(default) = payload.downcast_ref::<DefaultBehaviorFailure>() {
      return Self::new(default.clone());
    }

    if let Some(message) = payload.downcast_ref::<&str>() {
      return Self::from_message(format!("panic: {message}"));
    }

    if let Some(message) = payload.downcast_ref::<String>() {
      return Self::from_message(format!("panic: {message}"));
    }

    Self::new(DefaultBehaviorFailure::from_unknown_panic(payload))
  }

  /// Accessor to the wrapped [`BehaviorFailure`].
  #[must_use]
  pub fn behavior(&self) -> &dyn BehaviorFailure {
    &*self.inner
  }

  /// Human-readable description delegated to the inner failure.
  #[must_use]
  pub fn description(&self) -> Cow<'_, str> {
    self.inner.description()
  }
}

impl fmt::Debug for ActorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.behavior(), f)
  }
}

impl fmt::Display for ActorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let description = self.description();
    f.write_str(description.as_ref())
  }
}

impl<T> From<T> for ActorFailure
where
  T: BehaviorFailure,
{
  fn from(value: T) -> Self {
    Self::new(value)
  }
}

impl PartialEq for ActorFailure {
  fn eq(&self, other: &Self) -> bool {
    if ptr::eq(self.behavior(), other.behavior()) {
      return true;
    }
    self.description() == other.description()
  }
}

impl Eq for ActorFailure {}
