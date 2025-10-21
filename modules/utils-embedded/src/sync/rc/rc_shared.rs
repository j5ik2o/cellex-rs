use alloc::rc::Rc;
use core::ops::Deref;

use cellex_utils_core_rs::{
  MpscBackend, MpscHandle, QueueHandle, QueueStorage, RingBackend, RingHandle, Shared, StackBackend, StackHandle,
};

/// `Rc`-based shared reference wrapper.
///
/// Provides shared ownership using `Rc` in `no_std` environments.
/// Implements the `Shared` trait and can be used as handles for various collection backends.
///
/// # Features
///
/// - Reference counting via `Rc` (single-threaded only)
/// - Transparent access via `Deref`
/// - Ownership recovery via `try_unwrap`
#[derive(Debug)]
pub struct RcShared<T>(Rc<T>);

impl<T> RcShared<T> {
  /// Creates a new shared reference with the specified value.
  pub fn new(value: T) -> Self {
    Self(Rc::new(value))
  }

  /// Creates a shared reference from an existing `Rc`.
  pub const fn from_rc(rc: Rc<T>) -> Self {
    Self(rc)
  }

  /// Extracts the inner `Rc`.
  #[must_use]
  pub fn into_inner(self) -> Rc<T> {
    self.0
  }
}

impl<T> Clone for RcShared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> Deref for RcShared<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> Shared<T> for RcShared<T> {
  fn try_unwrap(self) -> Result<T, Self>
  where
    T: Sized, {
    Rc::try_unwrap(self.0).map_err(RcShared)
  }
}

impl<T, E> QueueHandle<E> for RcShared<T>
where
  T: QueueStorage<E>,
{
  type Storage = T;

  fn storage(&self) -> &Self::Storage {
    &self.0
  }
}

impl<T, B> MpscHandle<T> for RcShared<B>
where
  B: MpscBackend<T>,
{
  type Backend = B;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}

impl<E, B> RingHandle<E> for RcShared<B>
where
  B: RingBackend<E>,
{
  type Backend = B;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}

impl<T, B> StackHandle<T> for RcShared<B>
where
  B: StackBackend<T>,
{
  type Backend = B;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}
