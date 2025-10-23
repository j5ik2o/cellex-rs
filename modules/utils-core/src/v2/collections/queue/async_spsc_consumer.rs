use core::marker::PhantomData;

use crate::{
  sync::{
    async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
    ArcShared,
  },
  v2::collections::queue::backend::{AsyncQueueBackend, QueueError},
};

/// Async consumer for queues tagged with
/// [`SpscKey`](crate::v2::collections::queue::type_keys::SpscKey).
pub struct AsyncSpscConsumer<T, B, A = SpinAsyncMutex<B>>
where
  B: AsyncQueueBackend<T>,
  A: AsyncMutexLike<B>, {
  pub(crate) inner: ArcShared<A>,
  _pd:              PhantomData<(T, B)>,
}

impl<T, B, A> AsyncSpscConsumer<T, B, A>
where
  B: AsyncQueueBackend<T>,
  A: AsyncMutexLike<B>,
{
  pub(crate) fn new(inner: ArcShared<A>) -> Self {
    Self { inner, _pd: PhantomData }
  }

  /// Polls the next element from the queue.
  pub async fn poll(&self) -> Result<T, QueueError> {
    let mut guard = self.inner.lock().await;
    guard.poll().await
  }

  /// Signals that no more elements will be produced.
  pub async fn close(&self) {
    let mut guard = self.inner.lock().await;
    let _ = guard.close().await;
  }
}
