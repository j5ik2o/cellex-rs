use cellex_utils_core_rs::{
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared, Shared},
  v2::collections::queue::{
    backend::{OfferOutcome, OverflowPolicy, QueueError as V2QueueError, VecRingBackend},
    MpscQueue, VecRingStorage,
  },
  Element, QueueBase, QueueError, QueueRw, QueueSize,
};
use spin::Mutex;

type EntryShared<M> = ArcShared<Mutex<Option<M>>>;
type Backend<M> = VecRingBackend<EntryShared<M>>;
type Queue<M> = MpscQueue<EntryShared<M>, Backend<M>>;

#[derive(Clone, Copy)]
enum CapacityModel {
  Bounded(usize),
  Unbounded,
}

/// Compatibility layer that adapts v2 queues to the legacy `QueueRw` trait.
pub struct QueueRwCompat<M> {
  queue:          Queue<M>,
  capacity_model: CapacityModel,
}

impl<M> Clone for QueueRwCompat<M> {
  fn clone(&self) -> Self {
    let shared = self.queue.shared().clone();
    Self { queue: MpscQueue::new(shared), capacity_model: self.capacity_model }
  }
}

impl<M> QueueRwCompat<M> {
  /// Creates an unbounded queue that grows on demand.
  #[must_use]
  pub fn unbounded() -> Self {
    let storage = VecRingStorage::with_capacity(0);
    Self::from_parts(storage, OverflowPolicy::Grow, CapacityModel::Unbounded)
  }

  /// Creates a bounded queue with the specified capacity and overflow policy.
  #[must_use]
  pub fn bounded(capacity: usize, policy: OverflowPolicy) -> Self {
    let adjusted = capacity.max(1);
    let storage = VecRingStorage::with_capacity(adjusted);
    Self::from_parts(storage, policy, CapacityModel::Bounded(adjusted))
  }

  fn from_parts(storage: VecRingStorage<EntryShared<M>>, policy: OverflowPolicy, model: CapacityModel) -> Self {
    let backend = VecRingBackend::new_with_storage(storage, policy);
    let shared_backend = ArcShared::new(SpinSyncMutex::new(backend));
    let queue = MpscQueue::new(shared_backend);
    Self { queue, capacity_model: model }
  }

  fn reclaim(entry: EntryShared<M>) -> M {
    match ArcShared::try_unwrap(entry) {
      | Ok(mutex) => Self::take_from_option(mutex.into_inner()),
      | Err(entry) => {
        let taken = entry.lock().take();
        Self::take_from_option(taken)
      },
    }
  }

  fn take_from_option(value: Option<M>) -> M {
    match value {
      | Some(message) => message,
      | None => {
        debug_assert!(false, "attempted to reclaim an empty queue entry");
        unsafe { core::hint::unreachable_unchecked() }
      },
    }
  }

  fn map_offer_outcome(&self, entry: EntryShared<M>, outcome: OfferOutcome) -> Result<(), QueueError<M>> {
    match outcome {
      | OfferOutcome::Enqueued | OfferOutcome::DroppedOldest { .. } | OfferOutcome::GrewTo { .. } => {
        drop(entry);
        Ok(())
      },
      | OfferOutcome::DroppedNewest { .. } => Err(QueueError::Full(Self::reclaim(entry))),
    }
  }

  fn map_offer_error(entry: EntryShared<M>, error: V2QueueError) -> QueueError<M> {
    match error {
      | V2QueueError::Full => QueueError::Full(Self::reclaim(entry)),
      | V2QueueError::Closed => QueueError::Closed(Self::reclaim(entry)),
      | V2QueueError::WouldBlock => QueueError::OfferError(Self::reclaim(entry)),
      | V2QueueError::AllocError => QueueError::OfferError(Self::reclaim(entry)),
      | V2QueueError::Disconnected => QueueError::Disconnected,
      | V2QueueError::Empty => QueueError::OfferError(Self::reclaim(entry)),
    }
  }
}

impl<M> QueueBase<M> for QueueRwCompat<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    QueueSize::limited(self.queue.len())
  }

  fn capacity(&self) -> QueueSize {
    match self.capacity_model {
      | CapacityModel::Bounded(limit) => QueueSize::limited(limit),
      | CapacityModel::Unbounded => QueueSize::limitless(),
    }
  }
}

impl<M> QueueRw<M> for QueueRwCompat<M>
where
  M: Element,
{
  fn offer(&self, element: M) -> Result<(), QueueError<M>> {
    let entry = ArcShared::new(Mutex::new(Some(element)));
    let cloned = entry.clone();
    match self.queue.offer(cloned) {
      | Ok(outcome) => self.map_offer_outcome(entry, outcome),
      | Err(error) => Err(Self::map_offer_error(entry, error)),
    }
  }

  fn poll(&self) -> Result<Option<M>, QueueError<M>> {
    match self.queue.poll() {
      | Ok(entry) => Ok(Some(Self::reclaim(entry))),
      | Err(V2QueueError::Empty) => Ok(None),
      | Err(V2QueueError::Disconnected) | Err(V2QueueError::Closed) | Err(V2QueueError::Full) => {
        Err(QueueError::Disconnected)
      },
      | Err(V2QueueError::WouldBlock) | Err(V2QueueError::AllocError) => Err(QueueError::Disconnected),
    }
  }

  fn clean_up(&self) {
    let _ = self.queue.close();
  }
}
