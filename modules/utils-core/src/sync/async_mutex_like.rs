use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};
use async_trait::async_trait;

/// Async-aware mutex abstraction.
#[async_trait(?Send)]
pub trait AsyncMutexLike<T> {
  /// Guard type returned by [`AsyncMutexLike::lock`].
  type Guard<'a>: Deref<Target = T> + DerefMut where Self: 'a, T: 'a;

  /// Creates a new mutex instance wrapping the provided value.
  fn new(value: T) -> Self;

  /// Consumes the mutex and returns the inner value.
  fn into_inner(self) -> T;

  /// Asynchronously locks the mutex and yields a guard to the protected value.
  async fn lock(&self) -> Self::Guard<'_>;
}

#[async_trait(?Send)]
impl<T> AsyncMutexLike<T> for spin::Mutex<T> {
  type Guard<'a> = spin::MutexGuard<'a, T> where T: 'a;

  fn new(value: T) -> Self {
    Self::new(value)
  }

  fn into_inner(self) -> T {
    self.into_inner()
  }

  async fn lock(&self) -> Self::Guard<'_> {
    self.lock()
  }
}

/// Convenience alias for guards produced by [`AsyncMutexLike`].
pub type AsyncMutexLikeGuard<'a, M, T> = <M as AsyncMutexLike<T>>::Guard<'a>;
