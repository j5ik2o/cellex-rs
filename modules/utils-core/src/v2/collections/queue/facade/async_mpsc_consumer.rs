use core::marker::PhantomData;

use crate::{
  sync::{
    async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
    ArcShared,
  },
  v2::collections::queue::backend::{QueueBackend, QueueError},
};

/// Async consumer handle for queues tagged with
/// [`MpscKey`](crate::v2::collections::queue::type_keys::MpscKey).
pub struct AsyncMpscConsumer<T, B, A = SpinAsyncMutex<B>>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>, {
  pub(crate) inner: ArcShared<A>,
  _pd:              PhantomData<(T, B)>,
}

impl<T, B, A> AsyncMpscConsumer<T, B, A>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
{
  pub(crate) fn new(inner: ArcShared<A>) -> Self {
    Self { inner, _pd: PhantomData }
  }

  /// Polls the next element from the queue.
  pub async fn poll(&self) -> Result<T, QueueError> {
    let mut guard = self.inner.lock().await;
    guard.poll()
  }

  /// Signals that no more elements will be produced.
  pub async fn close(&self) {
    let mut guard = self.inner.lock().await;
    guard.close();
  }

  /// Returns the number of stored elements.
  #[must_use]
  pub async fn len(&self) -> usize {
    let guard = self.inner.lock().await;
    guard.len()
  }

  /// Returns the queue capacity.
  #[must_use]
  pub async fn capacity(&self) -> usize {
    let guard = self.inner.lock().await;
    guard.capacity()
  }

  /// Indicates whether the queue is empty.
  #[must_use]
  pub async fn is_empty(&self) -> bool {
    let guard = self.inner.lock().await;
    guard.is_empty()
  }

  /// Provides access to the shared backend.
  #[must_use]
  pub fn shared(&self) -> &ArcShared<A> {
    &self.inner
  }
}
