#![allow(missing_docs)]

use core::marker::PhantomData;

use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};

use cellex_actor_core_rs::MetricsSinkShared;
use cellex_actor_core_rs::{
  Mailbox, MailboxOptions, PriorityEnvelope, QueueMailbox, QueueMailboxProducer, QueueMailboxRecv,
};
use cellex_utils_embedded_rs::queue::priority::ArcPriorityQueue;
use cellex_utils_embedded_rs::queue::ring::ArcRingQueue;
use cellex_utils_embedded_rs::{
  Element, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter, DEFAULT_CAPACITY, PRIORITY_LEVELS,
};

use crate::arc_mailbox::ArcSignal;

pub struct ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex, {
  control: ArcPriorityQueue<PriorityEnvelope<M>, RM>,
  regular: ArcRingQueue<PriorityEnvelope<M>, RM>,
  regular_capacity: usize,
}

impl<M, RM> Clone for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self {
      control: self.control.clone(),
      regular: self.regular.clone(),
      regular_capacity: self.regular_capacity,
    }
  }
}

impl<M, RM> ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn new(_levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
    let control = ArcPriorityQueue::new(control_per_level).with_dynamic(control_per_level == 0);
    let regular = if regular_capacity == 0 {
      ArcRingQueue::new(DEFAULT_CAPACITY).with_dynamic(true)
    } else {
      ArcRingQueue::new(regular_capacity).with_dynamic(false)
    };

    Self {
      control,
      regular,
      regular_capacity,
    }
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
    let regular_cap = if self.regular_capacity == 0 {
      QueueSize::limitless()
    } else {
      QueueSize::limited(self.regular_capacity)
    };

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
    self.len()
  }

  fn capacity(&self) -> QueueSize {
    self.capacity()
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
    self.clean_up();
  }
}

impl<M, RM> QueueRw<PriorityEnvelope<M>> for ArcPriorityQueues<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.offer(envelope)
  }

  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.poll()
  }

  fn clean_up(&self) {
    self.clean_up();
  }
}

#[derive(Clone)]
pub struct ArcPriorityMailbox<M, RM = CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  inner: QueueMailbox<ArcPriorityQueues<M, RM>, ArcSignal<RM>>,
}

#[derive(Clone)]
pub struct ArcPriorityMailboxSender<M, RM = CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  inner: QueueMailboxProducer<ArcPriorityQueues<M, RM>, ArcSignal<RM>>,
}

#[derive(Debug)]
pub struct ArcPriorityMailboxRuntime<RM = CriticalSectionRawMutex>
where
  RM: RawMutex, {
  control_capacity_per_level: usize,
  regular_capacity: usize,
  levels: usize,
  _marker: PhantomData<RM>,
}

impl<RM> Default for ArcPriorityMailboxRuntime<RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self {
      control_capacity_per_level: DEFAULT_CAPACITY,
      regular_capacity: DEFAULT_CAPACITY,
      levels: PRIORITY_LEVELS,
      _marker: PhantomData,
    }
  }
}

impl<RM> ArcPriorityMailboxRuntime<RM>
where
  RM: RawMutex,
{
  pub const fn new(control_capacity_per_level: usize) -> Self {
    Self {
      control_capacity_per_level,
      regular_capacity: DEFAULT_CAPACITY,
      levels: PRIORITY_LEVELS,
      _marker: PhantomData,
    }
  }

  pub fn with_levels(mut self, levels: usize) -> Self {
    self.levels = levels.max(1);
    self
  }

  pub fn with_regular_capacity(mut self, capacity: usize) -> Self {
    self.regular_capacity = capacity;
    self
  }

  pub fn mailbox<M>(&self, options: MailboxOptions) -> (ArcPriorityMailbox<M, RM>, ArcPriorityMailboxSender<M, RM>)
  where
    M: Element, {
    let control_per_level = self.resolve_control_capacity(options.priority_capacity);
    let regular_capacity = self.resolve_regular_capacity(options.capacity);
    let queue = ArcPriorityQueues::new(self.levels, control_per_level, regular_capacity);
    let signal = ArcSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (
      ArcPriorityMailbox { inner: mailbox },
      ArcPriorityMailboxSender { inner: sender },
    )
  }

  fn resolve_control_capacity(&self, requested: QueueSize) -> usize {
    match requested {
      QueueSize::Limitless => self.control_capacity_per_level,
      QueueSize::Limited(value) => value,
    }
  }

  fn resolve_regular_capacity(&self, requested: QueueSize) -> usize {
    match requested {
      QueueSize::Limitless => self.regular_capacity,
      QueueSize::Limited(value) => value,
    }
  }
}

impl<RM> Clone for ArcPriorityMailboxRuntime<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self {
      control_capacity_per_level: self.control_capacity_per_level,
      regular_capacity: self.regular_capacity,
      levels: self.levels,
      _marker: PhantomData,
    }
  }
}

impl<M, RM> ArcPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  pub fn new(control_capacity_per_level: usize) -> (Self, ArcPriorityMailboxSender<M, RM>) {
    ArcPriorityMailboxRuntime::<RM>::new(control_capacity_per_level).mailbox(MailboxOptions::default())
  }

  pub fn inner(&self) -> &QueueMailbox<ArcPriorityQueues<M, RM>, ArcSignal<RM>> {
    &self.inner
  }

  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M, RM> Mailbox<PriorityEnvelope<M>> for ArcPriorityMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, ArcPriorityQueues<M, RM>, ArcSignal<RM>, PriorityEnvelope<M>>
  where
    Self: 'a;
  type SendError = QueueError<PriorityEnvelope<M>>;

  fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), Self::SendError> {
    self.inner.try_send(message)
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    self.inner.recv()
  }

  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }

  fn close(&self) {
    self.inner.close();
  }

  fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M, RM> ArcPriorityMailboxSender<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  pub fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.inner.try_send(message)
  }

  pub fn send(&self, message: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.inner.send(message)
  }

  pub fn try_send_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.try_send(PriorityEnvelope::new(message, priority))
  }

  pub fn send_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.send(PriorityEnvelope::new(message, priority))
  }

  pub fn try_send_control_with_priority(
    &self,
    message: M,
    priority: i8,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.try_send(PriorityEnvelope::control(message, priority))
  }

  pub fn send_control_with_priority(&self, message: M, priority: i8) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.send(PriorityEnvelope::control(message, priority))
  }

  pub fn inner(&self) -> &QueueMailboxProducer<ArcPriorityQueues<M, RM>, ArcSignal<RM>> {
    &self.inner
  }

  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

#[cfg(test)]
mod tests;
