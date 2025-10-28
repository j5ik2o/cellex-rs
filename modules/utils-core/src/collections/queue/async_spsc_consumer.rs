use core::marker::PhantomData;

use super::{
  async_queue::poll_shared,
  backend::{AsyncQueueBackend, QueueError},
};
use crate::sync::{
  async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
  ArcShared,
};

/// Async consumer for queues tagged with
/// [`SpscKey`](crate::collections::queue::type_keys::SpscKey).
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
  pub async fn poll(&self) -> Result<T, QueueError<T>> {
    poll_shared::<T, B, A>(&self.inner).await
  }

  /// Signals that no more elements will be produced.
  pub async fn close(&self) -> Result<(), QueueError<T>> {
    let mut guard = <A as AsyncMutexLike<B>>::lock(&*self.inner).await.map_err(QueueError::from)?;
    guard.close().await
  }
}
