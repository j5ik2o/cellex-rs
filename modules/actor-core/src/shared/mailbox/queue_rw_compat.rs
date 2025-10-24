use cellex_utils_core_rs::{
  collections::queue::{QueueError, QueueError as V2QueueError},
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared, Shared},
  v2::collections::queue::{
    backend::{OfferOutcome, OverflowPolicy, VecRingBackend},
    MpscQueue, VecRingStorage,
  },
  Element, QueueBase, QueueRw, QueueSize,
};
use spin::Mutex;

use crate::api::metrics::{MetricsEvent, MetricsSinkShared};

#[cfg(test)]
mod tests;

type EntryShared<M> = ArcShared<Mutex<Option<M>>>;
type Backend<M> = VecRingBackend<EntryShared<M>>;
type Queue<M> = MpscQueue<EntryShared<M>, Backend<M>>;
type MetricsBinding = ArcShared<SpinSyncMutex<Option<MetricsSinkShared>>>;

#[derive(Clone, Copy)]
enum CapacityModel {
  Bounded(usize),
  Unbounded,
}

/// Compatibility layer that adapts v2 queues to the legacy `QueueRw` trait.
pub struct QueueRwCompat<M> {
  queue:          Queue<M>,
  capacity_model: CapacityModel,
  metrics_sink:   MetricsBinding,
}

impl<M> Clone for QueueRwCompat<M> {
  fn clone(&self) -> Self {
    let shared = self.queue.shared().clone();
    Self {
      queue:          MpscQueue::new(shared),
      capacity_model: self.capacity_model,
      metrics_sink:   self.metrics_sink.clone(),
    }
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
    let metrics_sink = ArcShared::new(SpinSyncMutex::new(None));
    Self { queue, capacity_model: model, metrics_sink }
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
      | OfferOutcome::Enqueued => {
        drop(entry);
        Ok(())
      },
      | OfferOutcome::DroppedOldest { count } => {
        self.record_event(MetricsEvent::MailboxDroppedOldest { count });
        drop(entry);
        Ok(())
      },
      | OfferOutcome::DroppedNewest { count } => {
        self.record_event(MetricsEvent::MailboxDroppedNewest { count });
        Err(QueueError::Full(Self::reclaim(entry)))
      },
      | OfferOutcome::GrewTo { capacity } => {
        self.record_event(MetricsEvent::MailboxGrewTo { capacity });
        drop(entry);
        Ok(())
      },
    }
  }

  fn map_offer_error(entry: EntryShared<M>, error: V2QueueError<EntryShared<M>>) -> QueueError<M> {
    match error {
      | V2QueueError::Full(preserved) => QueueError::Full(Self::reclaim(preserved)),
      | V2QueueError::Closed(preserved) => QueueError::Closed(Self::reclaim(preserved)),
      | V2QueueError::AllocError(preserved) => QueueError::AllocError(Self::reclaim(preserved)),
      | V2QueueError::OfferError(preserved) => QueueError::OfferError(Self::reclaim(preserved)),
      | V2QueueError::WouldBlock | V2QueueError::Empty => QueueError::OfferError(Self::reclaim(entry)),
      | V2QueueError::Disconnected => QueueError::Disconnected,
    }
  }

  fn record_event(&self, event: MetricsEvent) {
    let sink = {
      let guard = self.metrics_sink.lock();
      guard.clone()
    };
    if let Some(sink) = sink {
      sink.with_ref(|sink| sink.record(event));
    }
  }

  /// Updates the metrics sink used for queue-level instrumentation.
  pub fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    let mut guard = self.metrics_sink.lock();
    *guard = sink;
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
      | Err(V2QueueError::Disconnected) => Err(QueueError::Disconnected),
      | Err(V2QueueError::WouldBlock) => Err(QueueError::WouldBlock),
      | Err(V2QueueError::AllocError(preserved)) => Err(QueueError::AllocError(Self::reclaim(preserved))),
      | Err(V2QueueError::OfferError(preserved)) => Err(QueueError::OfferError(Self::reclaim(preserved))),
      | Err(V2QueueError::Closed(preserved)) => Err(QueueError::Closed(Self::reclaim(preserved))),
      | Err(V2QueueError::Full(preserved)) => Err(QueueError::Full(Self::reclaim(preserved))),
    }
  }

  fn clean_up(&self) {
    let _ = self.queue.close();
  }
}
