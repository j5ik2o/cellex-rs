#![allow(clippy::disallowed_types)]
mod arc_state_cell;
#[cfg(test)]
mod tests;

use alloc::sync::Arc;

pub use arc_state_cell::{ArcCsStateCell, ArcLocalStateCell, ArcStateCell};
use cellex_utils_core_rs::{
  MpscBackend, MpscHandle, QueueHandle, QueueStorage, RingBackend, RingHandle, Shared, StackBackend, StackHandle,
};

/// `Arc`-based shared reference type for embedded environments.
///
/// This type wraps the standard library's `Arc` to provide thread-safe reference counting
/// for shared ownership of values in embedded systems. It implements the [`Shared`] trait
/// and various handle traits for integration with queues, stacks, and other data structures.
///
/// # Examples
///
/// ```
/// use cellex_utils_embedded_rs::sync::ArcShared;
///
/// let shared = ArcShared::new(42);
/// let clone = shared.clone();
/// assert_eq!(*shared, 42);
/// assert_eq!(*clone, 42);
/// ```
pub struct ArcShared<T: ?Sized>(Arc<T>);

impl<T: ?Sized> core::fmt::Debug for ArcShared<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("ArcShared").finish()
  }
}

impl<T> ArcShared<T>
where
  T: Sized,
{
  /// Creates a new `ArcShared` containing the given value.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::sync::ArcShared;
  ///
  /// let shared = ArcShared::new(42);
  /// assert_eq!(*shared, 42);
  /// ```
  pub fn new(value: T) -> Self {
    Self(Arc::new(value))
  }
}

impl<T: ?Sized> ArcShared<T> {
  /// Creates a new `ArcShared` from an existing `Arc`.
  ///
  /// This allows wrapping an already-allocated `Arc` without additional allocation.
  ///
  /// # Examples
  ///
  /// ```
  /// use alloc::sync::Arc;
  ///
  /// use cellex_utils_embedded_rs::sync::ArcShared;
  ///
  /// let arc = Arc::new(42);
  /// let shared = ArcShared::from_arc(arc);
  /// assert_eq!(*shared, 42);
  /// ```
  pub fn from_arc(inner: Arc<T>) -> Self {
    Self(inner)
  }

  /// Consumes this `ArcShared` and returns the underlying `Arc`.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::sync::ArcShared;
  ///
  /// let shared = ArcShared::new(42);
  /// let arc = shared.into_arc();
  /// assert_eq!(*arc, 42);
  /// ```
  pub fn into_arc(self) -> Arc<T> {
    self.0
  }
}

impl<T: ?Sized> Clone for ArcShared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T: ?Sized> core::ops::Deref for ArcShared<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T: ?Sized> Shared<T> for ArcShared<T> {
  fn try_unwrap(self) -> Result<T, Self>
  where
    T: Sized, {
    Arc::try_unwrap(self.0).map_err(ArcShared)
  }
}

impl<T, E> QueueHandle<E> for ArcShared<T>
where
  T: QueueStorage<E>,
{
  type Storage = T;

  fn storage(&self) -> &Self::Storage {
    &self.0
  }
}

impl<T, B> MpscHandle<T> for ArcShared<B>
where
  B: MpscBackend<T>,
{
  type Backend = B;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}

impl<E, B> RingHandle<E> for ArcShared<B>
where
  B: RingBackend<E> + ?Sized,
{
  type Backend = B;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}

impl<T, B> StackHandle<T> for ArcShared<B>
where
  B: StackBackend<T> + ?Sized,
{
  type Backend = B;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}
