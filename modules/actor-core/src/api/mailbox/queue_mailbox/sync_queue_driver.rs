use cellex_utils_core_rs::{
  collections::{queue::QueueSize, Element},
  sync::{shared::Shared, sync_mutex_like::SpinSyncMutex, ArcShared},
  v2::collections::queue::{
    backend::{OfferOutcome, OverflowPolicy, QueueError, VecRingBackend},
    storage::VecRingStorage,
    MpscQueue,
  },
};
use spin::Mutex;

use super::{MailboxQueueDriver, QueuePollOutcome};
use crate::api::{
  mailbox::MailboxOverflowPolicy,
  metrics::{MetricsEvent, MetricsSinkShared},
};

type EntryShared<M> = ArcShared<Mutex<Option<M>>>;
type Backend<M> = VecRingBackend<EntryShared<M>>;
type Queue<M> = MpscQueue<EntryShared<M>, Backend<M>>;
type MetricsBinding = ArcShared<SpinSyncMutex<Option<MetricsSinkShared>>>;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
enum CapacityModel {
  Bounded(usize),
  Unbounded,
}

/// Driver that owns a v2 `SyncQueue` and exposes the legacy mailbox interface.
pub struct SyncQueueDriver<M> {
  queue:          Queue<M>,
  capacity_model: CapacityModel,
  policy:         OverflowPolicy,
  metrics_sink:   MetricsBinding,
}

impl<M> Clone for SyncQueueDriver<M> {
  fn clone(&self) -> Self {
    let shared = self.queue.shared().clone();
    Self {
      queue:          MpscQueue::new(shared),
      capacity_model: self.capacity_model,
      policy:         self.policy,
      metrics_sink:   self.metrics_sink.clone(),
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
    Self { queue, capacity_model: model, policy, metrics_sink }
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

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    Some(match self.policy {
      | OverflowPolicy::DropNewest => MailboxOverflowPolicy::DropNewest,
      | OverflowPolicy::DropOldest => MailboxOverflowPolicy::DropOldest,
      | OverflowPolicy::Grow => MailboxOverflowPolicy::Grow,
      | OverflowPolicy::Block => MailboxOverflowPolicy::Block,
    })
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
        | QueueError::Full(preserved) => Err(QueueError::Full(Self::reclaim(preserved))),
        | QueueError::Closed(preserved) => Err(QueueError::Closed(Self::reclaim(preserved))),
        | QueueError::AllocError(preserved) => Err(QueueError::AllocError(Self::reclaim(preserved))),
        | QueueError::OfferError(preserved) => Err(QueueError::OfferError(Self::reclaim(preserved))),
        | QueueError::WouldBlock | QueueError::Empty => Err(QueueError::OfferError(Self::reclaim(entry))),
        | QueueError::Disconnected => Err(QueueError::Disconnected),
      },
    }
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    match self.queue.poll() {
      | Ok(entry) => Ok(QueuePollOutcome::Message(Self::reclaim(entry))),
      | Err(QueueError::Empty) => Ok(QueuePollOutcome::Empty),
      | Err(QueueError::WouldBlock) => Ok(QueuePollOutcome::Pending),
      | Err(QueueError::Disconnected) => Ok(QueuePollOutcome::Disconnected),
      | Err(QueueError::Closed(preserved)) => Ok(QueuePollOutcome::Closed(Self::reclaim(preserved))),
      | Err(QueueError::AllocError(preserved)) => Err(QueueError::AllocError(Self::reclaim(preserved))),
      | Err(QueueError::OfferError(preserved)) => Err(QueueError::OfferError(Self::reclaim(preserved))),
      | Err(QueueError::Full(preserved)) => Err(QueueError::Full(Self::reclaim(preserved))),
    }
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    match self.queue.close() {
      | Ok(()) => Ok(None),
      | Err(QueueError::Closed(preserved)) => Ok(Some(Self::reclaim(preserved))),
      | Err(QueueError::Disconnected) => Err(QueueError::Disconnected),
      | Err(QueueError::AllocError(preserved)) => Err(QueueError::AllocError(Self::reclaim(preserved))),
      | Err(QueueError::OfferError(preserved)) => Err(QueueError::OfferError(Self::reclaim(preserved))),
      | Err(QueueError::Full(preserved)) => Err(QueueError::Full(Self::reclaim(preserved))),
      | Err(QueueError::WouldBlock) | Err(QueueError::Empty) => Ok(None),
    }
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    let mut guard = self.metrics_sink.lock();
    *guard = sink;
  }
}
