use core::marker::PhantomData;

use crate::{
  sync::{
    async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
    ArcShared,
  },
  v2::collections::queue::backend::{OfferOutcome, QueueBackend, QueueError},
};

/// Async producer handle for queues tagged with
/// [`MpscKey`](crate::v2::collections::queue::type_keys::MpscKey).
pub struct AsyncMpscProducer<T, B, A = SpinAsyncMutex<B>>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>, {
  pub(crate) inner: ArcShared<A>,
  _pd:              PhantomData<(T, B)>,
}

impl<T, B, A> AsyncMpscProducer<T, B, A>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
{
  pub(crate) fn new(inner: ArcShared<A>) -> Self {
    Self { inner, _pd: PhantomData }
  }

  /// Offers an element to the queue using the underlying backend.
  pub async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError> {
    let mut guard = self.inner.lock().await;
    guard.offer(item)
  }

  /// Provides access to the shared backend.
  #[must_use]
  pub fn shared(&self) -> &ArcShared<A> {
    &self.inner
  }
}

impl<T, B, A> Clone for AsyncMpscProducer<T, B, A>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
{
  fn clone(&self) -> Self {
    Self::new(self.inner.clone())
  }
}
