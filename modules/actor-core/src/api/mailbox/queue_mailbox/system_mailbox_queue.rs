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

use super::{MailboxQueueBackend, QueuePollOutcome, UserMailboxQueue};
use crate::{
  api::{
    mailbox::{messages::SystemMessage, MailboxOverflowPolicy},
    metrics::{MetricsEvent, MetricsSinkShared},
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Mailbox queue wrapper that reserves capacity for system messages.
pub struct SystemMailboxQueue<M>
where
  M: Element, {
  base:   UserMailboxQueue<M>,
  system: Option<SystemLane<M>>,
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
    Self { base: self.base.clone(), system: self.system.clone() }
  }
}

impl<M> SystemMailboxQueue<M>
where
  M: Element,
{
  /// Wraps a [`UserMailboxQueue`] with an optional reservation lane for system messages.
  pub fn new(base: UserMailboxQueue<M>, reservation: Option<usize>) -> Self {
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

    let system = reservation.and_then(|capacity| {
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
    Self { base, system }
  }

  fn reservation_len(&self) -> usize {
    self.system.as_ref().map(|lane| lane.queue.lock().len()).unwrap_or(0)
  }

  fn record_event(&self, event: MetricsEvent) {
    if let Some(lane) = &self.system {
      let sink = lane.metrics_sink.lock().clone();
      if let Some(sink) = sink {
        sink.with_ref(|sink| sink.record(event));
      }
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

  /// Returns the underlying [`UserMailboxQueue`] without altering its state.
  pub fn into_inner(self) -> UserMailboxQueue<M> {
    self.base
  }
}

impl<M> MailboxQueueBackend<M> for SystemMailboxQueue<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    let base_len = self.base.len();
    if base_len.is_limitless() {
      return base_len;
    }
    let system_len = self.reservation_len();
    QueueSize::limited(base_len.to_usize().saturating_add(system_len))
  }

  fn capacity(&self) -> QueueSize {
    let base_capacity = self.base.capacity();
    if base_capacity.is_limitless() {
      return base_capacity;
    }
    let reservation = self.system.as_ref().map(|lane| lane.capacity).unwrap_or(0);
    QueueSize::limited(base_capacity.to_usize().saturating_add(reservation))
  }

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    self.base.overflow_policy()
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    if let Some(lane) = &self.system {
      if Self::is_system_message(&message) {
        let mut guard = lane.queue.lock();
        if guard.len() < lane.capacity {
          guard.push_back(message);
          let remaining = lane.capacity.saturating_sub(guard.len());
          drop(guard);
          self.record_event(MetricsEvent::MailboxSystemReservedUsed { remaining });
          return Ok(OfferOutcome::Enqueued);
        }
        drop(guard);
        self.record_event(MetricsEvent::MailboxSystemReservationExhausted);
        return Err(QueueError::Full(message));
      }
    }

    self.base.offer(message)
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    if let Some(lane) = &self.system {
      let mut guard = lane.queue.lock();
      if let Some(message) = guard.pop_front() {
        return Ok(QueuePollOutcome::Message(message));
      }
    }
    self.base.poll()
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    let mut preserved = None;
    if let Some(lane) = &self.system {
      let mut guard = lane.queue.lock();
      if let Some(message) = guard.pop_front() {
        preserved = Some(message);
      }
    }

    match self.base.close()? {
      | Some(message) if preserved.is_none() => preserved = Some(message),
      | _ => {},
    }

    Ok(preserved)
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    self.base.set_metrics_sink(sink.clone());
    if let Some(lane) = &self.system {
      let mut guard = lane.metrics_sink.lock();
      *guard = sink;
    }
  }
}
