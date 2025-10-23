use core::marker::PhantomData;

use crate::{
  sync::{
    async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
    ArcShared,
  },
  v2::collections::stack::{PushOutcome, StackBackend, StackError},
};

#[cfg(test)]
mod tests;

/// Async stack facade wrapping a shared backend guarded by an async-capable mutex.
#[derive(Clone)]
pub struct AsyncStack<T, B, A = SpinAsyncMutex<B>>
where
  B: StackBackend<T>,
  A: AsyncMutexLike<B>, {
  inner: ArcShared<A>,
  _pd:   PhantomData<(T, B)>,
}

impl<T, B, A> AsyncStack<T, B, A>
where
  B: StackBackend<T>,
  A: AsyncMutexLike<B>,
{
  /// Creates a new async stack from the provided shared backend.
  #[must_use]
  pub fn new(shared_backend: ArcShared<A>) -> Self {
    Self { inner: shared_backend, _pd: PhantomData }
  }

  /// Pushes an item onto the stack.
  pub async fn push(&self, item: T) -> Result<PushOutcome, StackError> {
    let mut guard = self.inner.lock().await;
    guard.push(item)
  }

  /// Pops the top item from the stack.
  pub async fn pop(&self) -> Result<T, StackError> {
    let mut guard = self.inner.lock().await;
    guard.pop()
  }

  /// Returns the top item without removing it.
  pub async fn peek(&self) -> Result<Option<T>, StackError>
  where
    T: Clone, {
    let guard = self.inner.lock().await;
    Ok(guard.peek().cloned())
  }

  /// Requests the backend to transition into the closed state.
  pub async fn close(&self) -> Result<(), StackError> {
    let mut guard = self.inner.lock().await;
    guard.close();
    Ok(())
  }

  /// Returns the number of stored elements.
  #[must_use]
  pub async fn len(&self) -> usize {
    let guard = self.inner.lock().await;
    guard.len()
  }

  /// Returns the storage capacity.
  #[must_use]
  pub async fn capacity(&self) -> usize {
    let guard = self.inner.lock().await;
    guard.capacity()
  }

  /// Indicates whether the stack is empty.
  #[must_use]
  pub async fn is_empty(&self) -> bool {
    let guard = self.inner.lock().await;
    guard.len() == 0
  }

  /// Indicates whether the stack is full.
  #[must_use]
  pub async fn is_full(&self) -> bool {
    let guard = self.inner.lock().await;
    guard.len() == guard.capacity()
  }

  /// Provides access to the underlying shared backend.
  #[must_use]
  pub fn shared(&self) -> &ArcShared<A> {
    &self.inner
  }
}
