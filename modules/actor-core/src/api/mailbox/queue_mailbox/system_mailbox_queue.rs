extern crate alloc;

use alloc::collections::VecDeque;
use core::any::TypeId;

use cellex_utils_core_rs::{
  collections::{
    queue::{
      backend::{OfferOutcome, QueueError},
      QueueSize,
    },
    Element,
  },
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
};

use super::{MailboxQueueBackend, QueuePollOutcome, SystemMailboxLane};
use crate::{
  api::{
    mailbox::messages::SystemMessage,
    metrics::{MetricsEvent, MetricsSinkShared},
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Queue that owns the reservation lane dedicated to system messages.
pub struct SystemMailboxQueue<M>
where
  M: Element, {
  lane: Option<SystemLane<M>>,
}

struct SystemLane<M>
where
  M: Element, {
  queue:        ArcShared<SpinSyncMutex<VecDeque<M>>>,
  capacity:     usize,
  metrics_sink: ArcShared<SpinSyncMutex<Option<MetricsSinkShared>>>,
}

impl<M> Clone for SystemLane<M>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { queue: self.queue.clone(), capacity: self.capacity, metrics_sink: self.metrics_sink.clone() }
  }
}

impl<M> Clone for SystemMailboxQueue<M>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { lane: self.lane.clone() }
  }
}

impl<M> SystemMailboxQueue<M>
where
  M: Element,
{
  /// Creates a new system mailbox queue with the specified reservation size.
  pub fn new(reservation: Option<usize>) -> Self {
    #[cfg(debug_assertions)]
    {
      use core::any::{type_name, TypeId};
      if reservation.is_some()
        && TypeId::of::<M>() != TypeId::of::<PriorityEnvelope<AnyMessage>>()
        && TypeId::of::<M>() != TypeId::of::<PriorityEnvelope<SystemMessage>>()
      {
        debug_assert!(
          false,
          "SystemMailboxQueue reservation is configured for unsupported message type: {}",
          type_name::<M>()
        );
      }
    }

    let lane = reservation.and_then(|capacity| {
      if capacity == 0 {
        None
      } else {
        Some(SystemLane {
          queue: ArcShared::new(SpinSyncMutex::new(VecDeque::with_capacity(capacity))),
          capacity,
          metrics_sink: ArcShared::new(SpinSyncMutex::new(None)),
        })
      }
    });

    Self { lane }
  }

  fn with_lane<R>(&self, mut f: impl FnMut(&SystemLane<M>) -> R) -> Option<R> {
    self.lane.as_ref().map(|lane| f(lane))
  }

  fn reserve_len(lane: &SystemLane<M>) -> usize {
    lane.queue.lock().len()
  }

  fn record_event(lane: &SystemLane<M>, event: MetricsEvent) {
    let sink = lane.metrics_sink.lock().clone();
    if let Some(sink) = sink {
      sink.with_ref(|sink| sink.record(event));
    }
  }

  fn is_system_message(message: &M) -> bool {
    let type_id = TypeId::of::<M>();
    if type_id == TypeId::of::<PriorityEnvelope<AnyMessage>>() {
      let envelope = unsafe { &*(message as *const M as *const PriorityEnvelope<AnyMessage>) };
      envelope.system_message().is_some()
    } else if type_id == TypeId::of::<PriorityEnvelope<SystemMessage>>() {
      true
    } else {
      false
    }
  }
}

impl<M> SystemMailboxLane<M> for SystemMailboxQueue<M>
where
  M: Element,
{
  fn accepts(&self, message: &M) -> bool {
    Self::is_system_message(message) && self.lane.is_some()
  }
}

impl<M> MailboxQueueBackend<M> for SystemMailboxQueue<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    self.with_lane(|lane| QueueSize::limited(Self::reserve_len(lane))).unwrap_or_else(|| QueueSize::limited(0))
  }

  fn capacity(&self) -> QueueSize {
    self.with_lane(|lane| QueueSize::limited(lane.capacity)).unwrap_or_else(|| QueueSize::limited(0))
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    match &self.lane {
      | Some(lane) => {
        let mut guard = lane.queue.lock();
        if guard.len() < lane.capacity {
          guard.push_back(message);
          let remaining = lane.capacity.saturating_sub(guard.len());
          drop(guard);
          Self::record_event(lane, MetricsEvent::MailboxSystemReservedUsed { remaining });
          Ok(OfferOutcome::Enqueued)
        } else {
          drop(guard);
          Self::record_event(lane, MetricsEvent::MailboxSystemReservationExhausted);
          Err(QueueError::Full(message))
        }
      },
      | None => Err(QueueError::Full(message)),
    }
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    match &self.lane {
      | Some(lane) => {
        let mut guard = lane.queue.lock();
        if let Some(message) = guard.pop_front() {
          Ok(QueuePollOutcome::Message(message))
        } else {
          Ok(QueuePollOutcome::Empty)
        }
      },
      | None => Ok(QueuePollOutcome::Empty),
    }
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    match &self.lane {
      | Some(lane) => {
        let mut guard = lane.queue.lock();
        Ok(guard.pop_front())
      },
      | None => Ok(None),
    }
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    if let Some(lane) = &self.lane {
      let mut guard = lane.metrics_sink.lock();
      *guard = sink;
    }
  }
}
