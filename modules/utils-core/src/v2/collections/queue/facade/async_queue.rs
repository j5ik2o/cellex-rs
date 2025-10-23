use core::marker::PhantomData;

use super::{
  async_mpsc_consumer::AsyncMpscConsumer, async_mpsc_producer::AsyncMpscProducer,
  async_spsc_consumer::AsyncSpscConsumer, async_spsc_producer::AsyncSpscProducer,
};
use crate::{
  sync::{
    async_mutex_like::{AsyncMutexLike, SpinAsyncMutex},
    ArcShared,
  },
  v2::collections::queue::{
    backend::{OfferOutcome, PriorityBackend, QueueBackend, QueueError},
    capabilities::{MultiProducer, SingleConsumer, SingleProducer, SupportsPeek},
    type_keys::{FifoKey, MpscKey, PriorityKey, SpscKey, TypeKey},
  },
};

#[cfg(test)]
mod tests;

/// Async queue facade wrapping a shared backend guarded by an async-capable mutex.
#[derive(Clone)]
pub struct AsyncQueue<T, K, B, A = SpinAsyncMutex<B>>
where
  K: TypeKey,
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>, {
  inner: ArcShared<A>,
  _pd:   PhantomData<(T, K, B)>,
}

impl<T, K, B, A> AsyncQueue<T, K, B, A>
where
  K: TypeKey,
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
{
  /// Creates a new async queue from the provided shared backend.
  #[must_use]
  pub fn new(shared_backend: ArcShared<A>) -> Self {
    Self { inner: shared_backend, _pd: PhantomData }
  }

  /// Adds an element to the queue according to the backend's policy.
  pub async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError> {
    let mut guard = self.inner.lock().await;
    guard.offer(item)
  }

  /// Removes and returns the next available item.
  pub async fn poll(&self) -> Result<T, QueueError> {
    let mut guard = self.inner.lock().await;
    guard.poll()
  }

  /// Requests the backend to transition into the closed state.
  pub async fn close(&self) -> Result<(), QueueError> {
    let mut guard = self.inner.lock().await;
    guard.close();
    Ok(())
  }

  /// Returns the current number of stored elements.
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

  /// Indicates whether the queue is empty.
  #[must_use]
  pub async fn is_empty(&self) -> bool {
    let guard = self.inner.lock().await;
    guard.is_empty()
  }

  /// Indicates whether the queue is full.
  #[must_use]
  pub async fn is_full(&self) -> bool {
    let guard = self.inner.lock().await;
    guard.is_full()
  }

  /// Provides access to the underlying shared backend.
  #[must_use]
  pub fn shared(&self) -> &ArcShared<A> {
    &self.inner
  }
}

impl<T, B, A> AsyncQueue<T, PriorityKey, B, A>
where
  T: Clone + Ord,
  B: PriorityBackend<T>,
  A: AsyncMutexLike<B>,
  PriorityKey: SupportsPeek,
{
  /// Retrieves the smallest element without removing it.
  pub async fn peek_min(&self) -> Result<Option<T>, QueueError> {
    let guard = self.inner.lock().await;
    Ok(guard.peek_min().cloned())
  }
}

impl<T, B, A> AsyncQueue<T, MpscKey, B, A>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
  MpscKey: MultiProducer + SingleConsumer,
{
  /// Creates an async queue tailored for MPSC usage.
  #[must_use]
  pub fn new_mpsc(shared_backend: ArcShared<A>) -> Self {
    Self::new(shared_backend)
  }

  /// Returns a producer handle that can be cloned and shared.
  #[must_use]
  pub fn producer_handle(&self) -> AsyncMpscProducer<T, B, A> {
    AsyncMpscProducer::new(self.inner.clone())
  }

  /// Consumes the queue and returns producer/consumer handles.
  pub fn into_mpsc_handles(self) -> (AsyncMpscProducer<T, B, A>, AsyncMpscConsumer<T, B, A>) {
    let consumer = AsyncMpscConsumer::new(self.inner.clone());
    let producer = AsyncMpscProducer::new(self.inner);
    (producer, consumer)
  }
}

impl<T, B, A> AsyncQueue<T, SpscKey, B, A>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
  SpscKey: SingleProducer + SingleConsumer,
{
  /// Creates an async queue tailored for SPSC usage.
  #[must_use]
  pub fn new_spsc(shared_backend: ArcShared<A>) -> Self {
    Self::new(shared_backend)
  }

  /// Consumes the queue and returns producer/consumer handles for SPSC usage.
  pub fn into_spsc_handles(self) -> (AsyncSpscProducer<T, B, A>, AsyncSpscConsumer<T, B, A>) {
    let consumer = AsyncSpscConsumer::new(self.inner.clone());
    let producer = AsyncSpscProducer::new(self.inner);
    (producer, consumer)
  }
}

impl<T, B, A> AsyncQueue<T, FifoKey, B, A>
where
  B: QueueBackend<T>,
  A: AsyncMutexLike<B>,
  FifoKey: SingleProducer + SingleConsumer,
{
  /// Creates an async queue tailored for FIFO usage.
  #[must_use]
  pub fn new_fifo(shared_backend: ArcShared<A>) -> Self {
    Self::new(shared_backend)
  }
}

/// Type alias for an async MPSC queue.
pub type AsyncMpscQueue<T, B, A = SpinAsyncMutex<B>> = AsyncQueue<T, MpscKey, B, A>;
/// Type alias for an async SPSC queue.
pub type AsyncSpscQueue<T, B, A = SpinAsyncMutex<B>> = AsyncQueue<T, SpscKey, B, A>;
/// Type alias for an async FIFO queue.
pub type AsyncFifoQueue<T, B, A = SpinAsyncMutex<B>> = AsyncQueue<T, FifoKey, B, A>;
/// Type alias for an async priority queue.
pub type AsyncPriorityQueue<T, B, A = SpinAsyncMutex<B>> = AsyncQueue<T, PriorityKey, B, A>;
