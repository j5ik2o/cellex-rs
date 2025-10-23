use core::marker::PhantomData;

use crate::{
  sync::{
    async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
    ArcShared,
  },
  v2::collections::queue::backend::{AsyncQueueBackend, OfferOutcome, QueueError},
};

/// Async producer for queues tagged with
/// [`SpscKey`](crate::v2::collections::queue::type_keys::SpscKey).
pub struct AsyncSpscProducer<T, B, A = SpinAsyncMutex<B>>
where
  B: AsyncQueueBackend<T>,
  A: AsyncMutexLike<B>, {
  pub(crate) inner: ArcShared<A>,
  _pd:              PhantomData<(T, B)>,
}

impl<T, B, A> AsyncSpscProducer<T, B, A>
where
  B: AsyncQueueBackend<T>,
  A: AsyncMutexLike<B>,
{
  pub(crate) fn new(inner: ArcShared<A>) -> Self {
    Self { inner, _pd: PhantomData }
  }

  /// Offers an element to the queue.
  pub async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError> {
    let mut guard = self.inner.lock().await;
    guard.offer(item).await
  }
}
