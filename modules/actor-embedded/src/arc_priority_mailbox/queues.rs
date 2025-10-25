#![cfg(feature = "queue-v1")]

use cellex_actor_core_rs::shared::mailbox::messages::PriorityEnvelope;
use cellex_utils_embedded_rs::{
  collections::queue::{priority::ArcPriorityQueue, ring::ArcRingQueue},
  Element, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter, DEFAULT_CAPACITY,
};
use embassy_sync::blocking_mutex::raw::RawMutex;

/// Priority queue bundle used by [`super::mailbox::ArcPriorityMailbox`].
pub struct ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex, {
  control:          ArcPriorityQueue<PriorityEnvelope<M>, RM>,
  regular:          ArcRingQueue<PriorityEnvelope<M>, RM>,
  regular_capacity: usize,
}

impl<M, RM> Clone for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self {
      control:          self.control.clone(),
      regular:          self.regular.clone(),
      regular_capacity: self.regular_capacity,
    }
  }
}

impl<M, RM> ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  pub fn new(_levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
    let control = ArcPriorityQueue::new(control_per_level).with_dynamic(control_per_level == 0);
    let regular = if regular_capacity == 0 {
      ArcRingQueue::new(DEFAULT_CAPACITY).with_dynamic(true)
    } else {
      ArcRingQueue::new(regular_capacity).with_dynamic(false)
    };

    Self { control, regular, regular_capacity }
  }

  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    if envelope.is_control() {
      self.control.offer(envelope)
    } else {
      if self.regular_capacity > 0 {
        let len = self.regular.len().to_usize();
        if len >= self.regular_capacity {
          return Err(QueueError::Full(envelope));
        }
      }
      self.regular.offer(envelope)
    }
  }

  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    if let Some(envelope) = self.control.poll()? {
      return Ok(Some(envelope));
    }
    self.regular.poll()
  }

  fn clean_up(&self) {
    self.control.clean_up();
    self.regular.clean_up();
  }

  fn len(&self) -> QueueSize {
    let control_len = self.control.len().to_usize();
    let regular_len = self.regular.len().to_usize();
    QueueSize::limited(control_len.saturating_add(regular_len))
  }

  fn capacity(&self) -> QueueSize {
    let control_cap = self.control.capacity();
    let regular_cap =
      if self.regular_capacity == 0 { QueueSize::limitless() } else { QueueSize::limited(self.regular_capacity) };

    if control_cap.is_limitless() || regular_cap.is_limitless() {
      QueueSize::limitless()
    } else {
      let total = control_cap.to_usize().saturating_add(regular_cap.to_usize());
      QueueSize::limited(total)
    }
  }
}

impl<M, RM> QueueBase<PriorityEnvelope<M>> for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn len(&self) -> QueueSize {
    ArcPriorityQueues::len(self)
  }

  fn capacity(&self) -> QueueSize {
    ArcPriorityQueues::capacity(self)
  }
}

impl<M, RM> QueueWriter<PriorityEnvelope<M>> for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn offer_mut(&mut self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.offer(envelope)
  }
}

impl<M, RM> QueueReader<PriorityEnvelope<M>> for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn poll_mut(&mut self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.poll()
  }

  fn clean_up_mut(&mut self) {
    ArcPriorityQueues::clean_up(self);
  }
}

impl<M, RM> QueueRw<PriorityEnvelope<M>> for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    ArcPriorityQueues::offer(self, envelope)
  }

  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    ArcPriorityQueues::poll(self)
  }

  fn clean_up(&self) {
    ArcPriorityQueues::clean_up(self);
  }
}
