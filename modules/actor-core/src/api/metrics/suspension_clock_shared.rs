use cellex_utils_core_rs::sync::{
  shared::{Shared, SharedBound},
  ArcShared,
};

use super::SuspensionClock;

/// Shared wrapper around a [`SuspensionClock`].
#[derive(Clone, Default)]
pub struct SuspensionClockShared {
  inner: Option<ArcShared<dyn SuspensionClock>>,
}

impl SuspensionClockShared {
  /// Creates an empty clock handle (no measurement).
  #[must_use]
  pub const fn null() -> Self {
    Self { inner: None }
  }

  /// Wraps a concrete clock instance.
  #[must_use]
  pub fn new<C>(clock: C) -> Self
  where
    C: SuspensionClock + SharedBound, {
    let shared = ArcShared::new(clock);
    let dyn_shared = shared.into_dyn(|inner| inner as &dyn SuspensionClock);
    Self { inner: Some(dyn_shared) }
  }

  /// Wraps an existing shared clock handle.
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn SuspensionClock>) -> Self {
    Self { inner: Some(inner) }
  }

  /// Returns the current timestamp if the clock is available.
  #[must_use]
  pub fn now(&self) -> Option<u64> {
    self.inner.as_ref().and_then(|clock| clock.with_ref(|clock| clock.now()))
  }

  /// Executes a closure with a reference to the underlying clock.
  pub fn with_ref<R>(&self, f: impl FnOnce(&dyn SuspensionClock) -> R) -> Option<R> {
    self.inner.as_ref().map(|clock| clock.with_ref(f))
  }

  /// Returns whether the handle wraps an actual clock implementation.
  #[must_use]
  pub fn is_some(&self) -> bool {
    self.inner.is_some()
  }
}
