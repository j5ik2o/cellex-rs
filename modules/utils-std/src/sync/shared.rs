#[cfg(test)]
mod tests;

use std::{ops::Deref, sync::Arc};

use cellex_utils_core_rs::{
  MpscBackend, MpscHandle, QueueHandle, QueueStorage, RingBackend, RingHandle, Shared, StackBackend, StackHandle,
};

/// Shared ownership wrapper using `Arc`
///
/// A type for safely sharing values across multiple threads.
/// Implements the `Shared` trait and various handle traits.
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
  /// Creates a new `ArcShared` from a value
  ///
  /// # Arguments
  ///
  /// * `value` - The value to share
  ///
  /// # Returns
  ///
  /// A new `ArcShared` instance
  pub fn new(value: T) -> Self {
    Self(Arc::new(value))
  }
}

impl<T: ?Sized> ArcShared<T> {
  /// Creates `ArcShared` from an existing `Arc`
  ///
  /// # Arguments
  ///
  /// * `inner` - An `Arc` instance
  ///
  /// # Returns
  ///
  /// An `ArcShared` instance wrapping the `Arc`
  pub fn from_arc(inner: Arc<T>) -> Self {
    Self(inner)
  }

  /// Converts `ArcShared` to the internal `Arc`
  ///
  /// # Returns
  ///
  /// The internal `Arc` instance
  pub fn into_arc(self) -> Arc<T> {
    self.0
  }
}

impl<T: ?Sized> Deref for ArcShared<T> {
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

impl<T: ?Sized> Clone for ArcShared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
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
  B: MpscBackend<T> + ?Sized,
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
