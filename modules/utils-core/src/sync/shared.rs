use core::ops::Deref;

/// Shared ownership abstraction used across runtimes.
pub trait Shared<T: ?Sized>: Clone + Deref<Target = T> {
  /// Attempt to unwrap the shared value. Implementations may override this to
  /// provide specialised behaviour (e.g. `Arc::try_unwrap`).
  ///
  /// # Errors
  ///
  /// Returns `Err(self)` if there are multiple owners of the value
  fn try_unwrap(self) -> Result<T, Self>
  where
    T: Sized, {
    Err(self)
  }

  /// Execute the provided closure with a shared reference to the inner value.
  fn with_ref<R>(&self, f: impl FnOnce(&T) -> R) -> R {
    f(self.deref())
  }
}

/// Extensions for shared handles that can be converted into trait objects.
pub trait SharedDyn<T: ?Sized>: Shared<T> {
  /// Shared wrapper yielded after converting to a new dynamically sized view.
  type Dyn<U: ?Sized + 'static>: Shared<U>;

  /// Converts the shared handle into another dynamically sized representation.
  fn into_dyn<U: ?Sized + 'static, F>(self, cast: F) -> Self::Dyn<U>
  where
    F: FnOnce(&T) -> &U;
}

/// Marker trait that expresses the synchronisation guarantees required for shared closures.
///
/// * On targets that provide atomic pointer operations (`target_has_atomic = "ptr"`), this marker
///   requires `Send + Sync`, matching the capabilities of `alloc::sync::Arc`.
/// * On targets without atomic support (e.g. RP2040), the marker imposes no additional bounds so
///   that `Rc`-backed implementations can be used safely in single-threaded contexts.
#[cfg(target_has_atomic = "ptr")]
pub trait SharedBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> SharedBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait used when atomic pointer support is unavailable.
pub trait SharedBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SharedBound for T {}
