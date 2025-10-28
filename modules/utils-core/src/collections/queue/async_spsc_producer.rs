use core::marker::PhantomData;

use super::{
  async_queue::offer_shared,
  backend::{AsyncQueueBackend, OfferOutcome, QueueError},
};
use crate::sync::{
  async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
  ArcShared,
};

/// Async producer for queues tagged with
/// [`SpscKey`](crate::collections::queue::type_keys::SpscKey).
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
  pub async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError<T>> {
    offer_shared::<T, B, A>(&self.inner, item).await
  }
}
