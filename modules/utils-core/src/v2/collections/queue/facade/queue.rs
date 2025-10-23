use core::marker::PhantomData;

use super::{MpscConsumer, MpscProducer, SpscConsumer, SpscProducer};
use crate::{
  sync::{
    sync_mutex_like::{SpinSyncMutex, SyncMutexLike},
    ArcShared, Shared,
  },
  v2::{
    collections::queue::{
      backend::{OfferOutcome, PriorityBackend, QueueBackend, QueueError},
      capabilities::{MultiProducer, SingleConsumer, SingleProducer, SupportsPeek},
      type_keys::{FifoKey, MpscKey, PriorityKey, SpscKey, TypeKey},
    },
    sync::SharedAccess,
  },
};

/// Queue facade parameterised by element type, type key, backend, and shared guard.
#[derive(Clone)]
pub struct Queue<T, K, B, M = SpinSyncMutex<B>>
where
  K: TypeKey,
  B: QueueBackend<T>,
  M: SyncMutexLike<B>, {
  inner: ArcShared<M>,
  _pd:   PhantomData<(T, K, B)>,
}

impl<T, K, B, M> Queue<T, K, B, M>
where
  K: TypeKey,
  B: QueueBackend<T>,
  M: SyncMutexLike<B>,
  ArcShared<M>: SharedAccess<B>,
{
  /// Creates a new queue from the provided shared backend.
  #[must_use]
  pub fn new(shared_backend: ArcShared<M>) -> Self {
    Self { inner: shared_backend, _pd: PhantomData }
  }

  /// Enqueues an item according to the backend's overflow policy.
  pub fn offer(&self, item: T) -> Result<OfferOutcome, QueueError> {
    self.inner.with_mut(|backend: &mut B| backend.offer(item)).map_err(QueueError::from)?
  }

  /// Dequeues the next available item.
  pub fn poll(&self) -> Result<T, QueueError> {
    self.inner.with_mut(|backend: &mut B| backend.poll()).map_err(QueueError::from)?
  }

  /// Requests the backend to transition into the closed state.
  pub fn close(&self) -> Result<(), QueueError> {
    self
      .inner
      .with_mut(|backend: &mut B| {
        backend.close();
        Ok(())
      })
      .map_err(QueueError::from)?
  }

  /// Returns the current number of stored elements.
  #[must_use]
  pub fn len(&self) -> usize {
    self.inner.with_ref(|mutex: &M| {
      let guard = mutex.lock();
      guard.len()
    })
  }

  /// Returns the storage capacity.
  #[must_use]
  pub fn capacity(&self) -> usize {
    self.inner.with_ref(|mutex: &M| {
      let guard = mutex.lock();
      guard.capacity()
    })
  }

  /// Indicates whether the queue is empty.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Indicates whether the queue is full.
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.len() == self.capacity()
  }

  /// Provides access to the underlying shared backend.
  #[must_use]
  pub fn shared(&self) -> &ArcShared<M> {
    &self.inner
  }
}

impl<T, B, M> Queue<T, PriorityKey, B, M>
where
  T: Clone + Ord,
  B: PriorityBackend<T>,
  M: SyncMutexLike<B>,
  ArcShared<M>: SharedAccess<B>,
  PriorityKey: SupportsPeek,
{
  /// Retrieves the smallest element without removing it.
  pub fn peek_min(&self) -> Result<Option<T>, QueueError> {
    self.inner.with_mut(|backend: &mut B| Ok(backend.peek_min().cloned())).map_err(QueueError::from)?
  }
}

impl<T, B, M> Queue<T, MpscKey, B, M>
where
  B: QueueBackend<T>,
  M: SyncMutexLike<B>,
  ArcShared<M>: SharedAccess<B>,
  MpscKey: MultiProducer + SingleConsumer,
{
  /// Creates a queue tailored for MPSC usage.
  #[must_use]
  pub fn new_mpsc(shared_backend: ArcShared<M>) -> Self {
    Queue::new(shared_backend)
  }

  /// Returns a producer handle that can be cloned and shared.
  #[must_use]
  pub fn producer_handle(&self) -> MpscProducer<T, B, M> {
    MpscProducer::new(self.inner.clone())
  }

  /// Consumes the queue and returns producer/consumer handles.
  pub fn into_mpsc_handles(self) -> (MpscProducer<T, B, M>, MpscConsumer<T, B, M>) {
    let consumer = MpscConsumer::new(self.inner.clone());
    let producer = MpscProducer::new(self.inner);
    (producer, consumer)
  }
}

impl<T, B, M> Queue<T, SpscKey, B, M>
where
  B: QueueBackend<T>,
  M: SyncMutexLike<B>,
  ArcShared<M>: SharedAccess<B>,
  SpscKey: SingleProducer + SingleConsumer,
{
  /// Creates a queue tailored for SPSC usage.
  #[must_use]
  pub fn new_spsc(shared_backend: ArcShared<M>) -> Self {
    Queue::new(shared_backend)
  }

  /// Consumes the queue and returns producer/consumer handles for SPSC usage.
  pub fn into_spsc_handles(self) -> (SpscProducer<T, B, M>, SpscConsumer<T, B, M>) {
    let consumer = SpscConsumer::new(self.inner.clone());
    let producer = SpscProducer::new(self.inner);
    (producer, consumer)
  }
}

impl<T, B, M> Queue<T, FifoKey, B, M>
where
  B: QueueBackend<T>,
  M: SyncMutexLike<B>,
  ArcShared<M>: SharedAccess<B>,
  FifoKey: SingleProducer + SingleConsumer,
{
  /// Creates a queue tailored for FIFO usage.
  #[must_use]
  pub fn new_fifo(shared_backend: ArcShared<M>) -> Self {
    Queue::new(shared_backend)
  }
}

/// Type alias for an MPSC queue.
pub type MpscQueue<T, B, M = SpinSyncMutex<B>> = Queue<T, MpscKey, B, M>;
/// Type alias for an SPSC queue.
pub type SpscQueue<T, B, M = SpinSyncMutex<B>> = Queue<T, SpscKey, B, M>;
/// Type alias for a FIFO queue.
pub type FifoQueue<T, B, M = SpinSyncMutex<B>> = Queue<T, FifoKey, B, M>;
/// Type alias for a priority queue.
pub type PriorityQueue<T, B, M = SpinSyncMutex<B>> = Queue<T, PriorityKey, B, M>;
