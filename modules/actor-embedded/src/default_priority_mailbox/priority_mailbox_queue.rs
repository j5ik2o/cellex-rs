extern crate alloc;

use alloc::vec::Vec;

use cellex_actor_core_rs::{
  api::{
    mailbox::{
      queue_mailbox::{MailboxQueueBackend, QueuePollOutcome, SyncMailboxQueue},
      MailboxOverflowPolicy,
    },
    metrics::MetricsSinkShared,
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_core_rs::collections::{
  queue::{
    backend::{OfferOutcome, OverflowPolicy, QueueError},
    QueueSize,
  },
  Element,
};

#[cfg(test)]
mod tests;

/// Multiplexes multiple `SyncMailboxQueue` instances and routes
/// `PriorityEnvelope` messages to either control or regular lanes.
pub struct PriorityMailboxQueue<M>
where
  M: Element, {
  control_lanes: Vec<SyncMailboxQueue<PriorityEnvelope<M>>>,
  regular_lane:  SyncMailboxQueue<PriorityEnvelope<M>>,
}

impl<M> Clone for PriorityMailboxQueue<M>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { control_lanes: self.control_lanes.clone(), regular_lane: self.regular_lane.clone() }
  }
}

impl<M> PriorityMailboxQueue<M>
where
  M: Element,
{
  /// Creates a driver with the requested number of priority levels.
  /// When a capacity value is `0`, the lane grows with `OverflowPolicy::Grow`.
  pub fn new(levels: usize, control_capacity_per_level: usize, regular_capacity: usize) -> Self {
    let levels = levels.max(1);
    let control_lanes = (0..levels).map(|_| make_lane(control_capacity_per_level)).collect();
    let regular_lane = make_lane(regular_capacity);
    Self { control_lanes, regular_lane }
  }

  fn control_level_index(&self, priority: i8) -> usize {
    let max = (self.control_lanes.len().saturating_sub(1)) as i8;
    priority.clamp(0, max) as usize
  }

  fn poll_control_lanes(
    &self,
  ) -> Result<Option<QueuePollOutcome<PriorityEnvelope<M>>>, QueueError<PriorityEnvelope<M>>> {
    let mut saw_pending = false;
    for lane in self.control_lanes.iter().rev() {
      match lane.poll()? {
        | QueuePollOutcome::Message(message) => return Ok(Some(QueuePollOutcome::Message(message))),
        | QueuePollOutcome::Empty => {},
        | QueuePollOutcome::Pending => saw_pending = true,
        | outcome @ QueuePollOutcome::Disconnected
        | outcome @ QueuePollOutcome::Closed(_)
        | outcome @ QueuePollOutcome::Err(_) => return Ok(Some(outcome)),
      }
    }
    if saw_pending {
      return Ok(Some(QueuePollOutcome::Pending));
    }
    Ok(None)
  }

  fn aggregate_capacity(&self) -> QueueSize {
    let mut total = 0usize;
    for lane in &self.control_lanes {
      let capacity = lane.capacity();
      if capacity.is_limitless() {
        return QueueSize::limitless();
      }
      total = total.saturating_add(capacity.to_usize());
    }
    let regular_capacity = self.regular_lane.capacity();
    if regular_capacity.is_limitless() {
      return QueueSize::limitless();
    }
    QueueSize::limited(total.saturating_add(regular_capacity.to_usize()))
  }

  fn aggregate_len(&self) -> QueueSize {
    let mut total = 0usize;
    for lane in &self.control_lanes {
      total = total.saturating_add(lane.len().to_usize());
    }
    total = total.saturating_add(self.regular_lane.len().to_usize());
    QueueSize::limited(total)
  }
}

impl<M> MailboxQueueBackend<PriorityEnvelope<M>> for PriorityMailboxQueue<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    self.aggregate_len()
  }

  fn capacity(&self) -> QueueSize {
    self.aggregate_capacity()
  }

  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<OfferOutcome, QueueError<PriorityEnvelope<M>>> {
    if envelope.is_control() {
      let idx = self.control_level_index(envelope.priority());
      self.control_lanes[idx].offer(envelope)
    } else {
      self.regular_lane.offer(envelope)
    }
  }

  fn poll(&self) -> Result<QueuePollOutcome<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    if let Some(outcome) = self.poll_control_lanes()? {
      if matches!(outcome, QueuePollOutcome::Pending) {
        match self.regular_lane.poll()? {
          | QueuePollOutcome::Empty => return Ok(QueuePollOutcome::Pending),
          | QueuePollOutcome::Pending => return Ok(QueuePollOutcome::Pending),
          | outcome => return Ok(outcome),
        }
      }
      return Ok(outcome);
    }

    match self.regular_lane.poll()? {
      | QueuePollOutcome::Empty => Ok(QueuePollOutcome::Empty),
      | outcome => Ok(outcome),
    }
  }

  fn close(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    let mut preserved: Option<PriorityEnvelope<M>> = None;

    for lane in &self.control_lanes {
      match lane.close()? {
        | Some(message) => {
          if preserved.is_none() {
            preserved = Some(message);
          }
        },
        | None => {},
      }
    }

    match self.regular_lane.close()? {
      | Some(message) if preserved.is_none() => preserved = Some(message),
      | _ => {},
    }

    Ok(preserved)
  }

  fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
    for lane in &self.control_lanes {
      lane.set_metrics_sink(sink.clone());
    }
    self.regular_lane.set_metrics_sink(sink);
  }

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    self.regular_lane.overflow_policy().or_else(|| self.control_lanes.first().and_then(|lane| lane.overflow_policy()))
  }
}

fn make_lane<M>(capacity: usize) -> SyncMailboxQueue<PriorityEnvelope<M>>
where
  M: Element, {
  match capacity {
    | 0 => SyncMailboxQueue::unbounded(),
    | limit => SyncMailboxQueue::bounded(limit, OverflowPolicy::Block),
  }
}
