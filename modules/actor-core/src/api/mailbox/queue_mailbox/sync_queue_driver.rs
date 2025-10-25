use cellex_utils_core_rs::{
  collections::queue::{QueueError, QueueError as V2QueueError},
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared, Shared},
  v2::collections::queue::{
    backend::{OfferOutcome, OverflowPolicy, VecRingBackend},
    MpscQueue, VecRingStorage,
  },
  Element, QueueSize,
};
use spin::Mutex;

use super::{MailboxQueueDriver, QueuePollOutcome};
use crate::api::metrics::{MetricsEvent, MetricsSinkShared};

type EntryShared<M> = ArcShared<Mutex<Option<M>>>;
type Backend<M> = VecRingBackend<EntryShared<M>>;
type Queue<M> = MpscQueue<EntryShared<M>, Backend<M>>;
type MetricsBinding = ArcShared<SpinSyncMutex<Option<MetricsSinkShared>>>;

enum CapacityModel {
  Bounded(usize),
  Unbounded,
}

/// Driver that owns a v2 `SyncQueue` and exposes the legacy mailbox interface.
pub struct SyncQueueDriver<M> {
  queue:          Queue<M>,
  capacity_model: CapacityModel,
  metrics_sink:   MetricsBinding,
}

impl<M> Clone for SyncQueueDriver<M> {
  fn clone(&self) -> Self {
    let shared = self.queue.shared().clone();
    Self {
      queue:          MpscQueue::new(shared),
      capacity_model: self.capacity_model.clone(),
      metrics_sink:   self.metrics_sink.clone(),
    }
  }
}

impl Clone for CapacityModel {
  fn clone(&self) -> Self {
    match self {
      | CapacityModel::Bounded(value) => CapacityModel::Bounded(*value),
      | CapacityModel::Unbounded => CapacityModel::Unbounded,
    }
  }
}

impl<M> SyncQueueDriver<M> {
  /// Creates an unbounded driver that grows the underlying storage as needed.
  pub fn unbounded() -> Self {
    let storage = VecRingStorage::with_capacity(0);
    Self::from_parts(storage, OverflowPolicy::Grow, CapacityModel::Unbounded)
  }

  /// Creates a bounded driver with the specified capacity and overflow policy.
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

  fn record_event(&self, event: MetricsEvent) {
    let sink = {
      let guard = self.metrics_sink.lock();
      guard.clone()
    };
    if let Some(sink) = sink {
      sink.with_ref(|sink| sink.record(event));
    }
  }
}

impl<M> MailboxQueueDriver<M> for SyncQueueDriver<M>
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

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    let entry = ArcShared::new(Mutex::new(Some(message)));
    let cloned = entry.clone();
    match self.queue.offer(cloned) {
      | Ok(outcome) => match outcome {
        | OfferOutcome::Enqueued => {
          drop(entry);
          Ok(OfferOutcome::Enqueued)
        },
        | OfferOutcome::DroppedOldest { count } => {
          self.record_event(MetricsEvent::MailboxDroppedOldest { count });
          drop(entry);
          Ok(OfferOutcome::DroppedOldest { count })
        },
        | OfferOutcome::DroppedNewest { count } => {
          self.record_event(MetricsEvent::MailboxDroppedNewest { count });
          Err(QueueError::Full(Self::reclaim(entry)))
        },
        | OfferOutcome::GrewTo { capacity } => {
          self.record_event(MetricsEvent::MailboxGrewTo { capacity });
          drop(entry);
          Ok(OfferOutcome::GrewTo { capacity })
        },
      },
      | Err(error) => match error {
        | V2QueueError::Full(preserved) => Err(QueueError::Full(Self::reclaim(preserved))),
        | V2QueueError::Closed(preserved) => Err(QueueError::Closed(Self::reclaim(preserved))),
        | V2QueueError::AllocError(preserved) => Err(QueueError::AllocError(Self::reclaim(preserved))),
        | V2QueueError::OfferError(preserved) => Err(QueueError::OfferError(Self::reclaim(preserved))),
        | V2QueueError::WouldBlock | V2QueueError::Empty => Err(QueueError::OfferError(Self::reclaim(entry))),
        | V2QueueError::Disconnected => Err(QueueError::Disconnected),
      },
    }
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    match self.queue.poll() {
      | Ok(entry) => Ok(QueuePollOutcome::Message(Self::reclaim(entry))),
      | Err(V2QueueError::Empty) => Ok(QueuePollOutcome::Empty),
      | Err(V2QueueError::WouldBlock) => Ok(QueuePollOutcome::Pending),
      | Err(V2QueueError::Disconnected) => Ok(QueuePollOutcome::Disconnected),
      | Err(V2QueueError::Closed(preserved)) => Ok(QueuePollOutcome::Closed(Self::reclaim(preserved))),
      | Err(V2QueueError::AllocError(preserved)) => Err(QueueError::AllocError(Self::reclaim(preserved))),
      | Err(V2QueueError::OfferError(preserved)) => Err(QueueError::OfferError(Self::reclaim(preserved))),
      | Err(V2QueueError::Full(preserved)) => Err(QueueError::Full(Self::reclaim(preserved))),
    }
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    match self.queue.close() {
      | Ok(()) => Ok(None),
      | Err(V2QueueError::Closed(preserved)) => Ok(Some(Self::reclaim(preserved))),
      | Err(V2QueueError::Disconnected) => Err(QueueError::Disconnected),
      | Err(V2QueueError::AllocError(preserved)) => Err(QueueError::AllocError(Self::reclaim(preserved))),
      | Err(V2QueueError::OfferError(preserved)) => Err(QueueError::OfferError(Self::reclaim(preserved))),
      | Err(V2QueueError::Full(preserved)) => Err(QueueError::Full(Self::reclaim(preserved))),
      | Err(V2QueueError::WouldBlock) | Err(V2QueueError::Empty) => Ok(None),
    }
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    let mut guard = self.metrics_sink.lock();
    *guard = sink;
  }
}
