use core::ops::{Deref, DerefMut};

/// Generic mutex abstraction for runtime-agnostic code.
pub trait MutexLike<T> {
  /// Guard type returned by [`MutexLike::lock`].
  type Guard<'a>: Deref<Target = T> + DerefMut where Self: 'a, T: 'a;

  /// Creates a new mutex instance wrapping the provided value.
  fn new(value: T) -> Self;

  /// Consumes the mutex and returns the inner value.
  fn into_inner(self) -> T;

  /// Locks the mutex and returns a guard to the protected value.
  fn lock(&self) -> Self::Guard<'_>;
}

// spin::Mutex implementation
impl<T> MutexLike<T> for spin::Mutex<T> {
  type Guard<'a> = spin::MutexGuard<'a, T> where T: 'a;

  fn new(value: T) -> Self {
    Self::new(value)
  }

  fn into_inner(self) -> T {
    self.into_inner()
  }

  fn lock(&self) -> Self::Guard<'_> {
    self.lock()
  }
}

/// Convenience alias for guards produced by [`MutexLike`].
pub type MutexLikeGuard<'a, M, T> = <M as MutexLike<T>>::Guard<'a>;
