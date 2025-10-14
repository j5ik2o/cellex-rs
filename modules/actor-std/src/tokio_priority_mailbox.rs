#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cellex_actor_core_rs::MetricsSinkShared;
use cellex_actor_core_rs::{
  Mailbox, MailboxOptions, PriorityEnvelope, QueueMailbox, QueueMailboxProducer, QueueMailboxRecv,
};
use cellex_utils_std_rs::{
  Element, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter, DEFAULT_CAPACITY, PRIORITY_LEVELS,
};

type PriorityQueueError<M> = Box<QueueError<PriorityEnvelope<M>>>;

use crate::tokio_mailbox::NotifySignal;

struct TokioPriorityLevels<M> {
  levels: Arc<Vec<Mutex<VecDeque<PriorityEnvelope<M>>>>>,
  capacity_per_level: usize,
}

impl<M> Clone for TokioPriorityLevels<M> {
  fn clone(&self) -> Self {
    Self {
      levels: Arc::clone(&self.levels),
      capacity_per_level: self.capacity_per_level,
    }
  }
}

impl<M> TokioPriorityLevels<M> {
  fn new(levels: usize, capacity_per_level: usize) -> Self {
    let storage = (0..levels).map(|_| Mutex::new(VecDeque::new())).collect();
    Self {
      levels: Arc::new(storage),
      capacity_per_level,
    }
  }

  fn level_index(priority: i8, levels: usize) -> usize {
    let max = (levels.saturating_sub(1)) as i8;
    priority.clamp(0, max) as usize
  }

  #[allow(clippy::result_large_err)]
  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    let idx = Self::level_index(envelope.priority(), self.levels.len());
    let mut guard = self.levels[idx].lock().expect("priority queue poisoned");
    if self.capacity_per_level > 0 && guard.len() >= self.capacity_per_level {
      Err(QueueError::Full(envelope))
    } else {
      guard.push_back(envelope);
      Ok(())
    }
  }

  #[allow(clippy::result_large_err)]
  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    for level in (0..self.levels.len()).rev() {
      let mut guard = self.levels[level].lock().expect("priority queue poisoned");
      if let Some(envelope) = guard.pop_front() {
        return Ok(Some(envelope));
      }
    }
    Ok(None)
  }

  fn clean_up(&self) {
    for level in self.levels.iter() {
      let mut guard = level.lock().expect("priority queue poisoned");
      guard.clear();
    }
  }

  fn len(&self) -> usize {
    self
      .levels
      .iter()
      .map(|level| level.lock().expect("priority queue poisoned").len())
      .sum()
  }

  fn capacity(&self) -> QueueSize {
    if self.capacity_per_level == 0 {
      QueueSize::limitless()
    } else {
      let levels = self.levels.len().max(1);
      QueueSize::limited(self.capacity_per_level * levels)
    }
  }
}

pub struct TokioPriorityQueues<M> {
  control: TokioPriorityLevels<M>,
  regular: Arc<Mutex<VecDeque<PriorityEnvelope<M>>>>,
  regular_capacity: usize,
}

impl<M> TokioPriorityQueues<M> {
  fn new(levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
    Self {
      control: TokioPriorityLevels::new(levels, control_per_level),
      regular: Arc::new(Mutex::new(VecDeque::new())),
      regular_capacity,
    }
  }

  #[allow(clippy::result_large_err)]
  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    if envelope.is_control() {
      self.control.offer(envelope)
    } else {
      let mut guard = self.regular.lock().expect("regular queue poisoned");
      if self.regular_capacity > 0 && guard.len() >= self.regular_capacity {
        Err(QueueError::Full(envelope))
      } else {
        guard.push_back(envelope);
        Ok(())
      }
    }
  }

  #[allow(clippy::result_large_err)]
  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    if let Some(envelope) = self.control.poll()? {
      return Ok(Some(envelope));
    }
    let mut guard = self.regular.lock().expect("regular queue poisoned");
    Ok(guard.pop_front())
  }

  fn clean_up(&self) {
    self.control.clean_up();
    let mut guard = self.regular.lock().expect("regular queue poisoned");
    guard.clear();
  }

  fn len(&self) -> QueueSize {
    let control_len = self.control.len();
    let regular_len = self.regular.lock().expect("regular queue poisoned").len();
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

impl<M> Clone for TokioPriorityQueues<M> {
  fn clone(&self) -> Self {
    Self {
      control: self.control.clone(),
      regular: Arc::clone(&self.regular),
      regular_capacity: self.regular_capacity,
    }
  }
}

impl<M> QueueBase<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn len(&self) -> QueueSize {
    self.len()
  }

  fn capacity(&self) -> QueueSize {
    self.capacity()
  }
}

impl<M> QueueWriter<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn offer_mut(&mut self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.offer(envelope)
  }
}

impl<M> QueueReader<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn poll_mut(&mut self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.poll()
  }

  fn clean_up_mut(&mut self) {
    self.clean_up();
  }
}

impl<M> QueueRw<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
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

/// Priority mailbox for Tokio runtime
///
/// An asynchronous mailbox that processes messages based on priority.
/// Control messages are processed with higher priority than regular messages.
pub struct TokioPriorityMailbox<M>
where
  M: Element, {
  inner: QueueMailbox<TokioPriorityQueues<M>, NotifySignal>,
}

/// Message sender handle for priority mailbox
///
/// Provides an asynchronous interface for sending messages to the mailbox.
/// Supports sending messages with specified priority and control messages.
pub struct TokioPriorityMailboxSender<M>
where
  M: Element, {
  inner: QueueMailboxProducer<TokioPriorityQueues<M>, NotifySignal>,
}

/// Factory that creates priority mailboxes
///
/// Configures the capacity of control and regular queues and the number of priority levels,
/// and creates mailbox instances.
#[derive(Clone, Debug)]
pub struct TokioPriorityMailboxRuntime {
  control_capacity_per_level: usize,
  regular_capacity: usize,
  levels: usize,
}

impl Default for TokioPriorityMailboxRuntime {
  fn default() -> Self {
    Self {
      control_capacity_per_level: DEFAULT_CAPACITY,
      regular_capacity: DEFAULT_CAPACITY,
      levels: PRIORITY_LEVELS,
    }
  }
}

impl TokioPriorityMailboxRuntime {
  /// Creates a new factory instance
  ///
  /// # Arguments
  ///
  /// * `control_capacity_per_level` - Capacity per priority level for the control queue
  ///
  /// # Returns
  ///
  /// A factory initialized with default regular queue capacity and default number of priority levels
  pub fn new(control_capacity_per_level: usize) -> Self {
    Self {
      control_capacity_per_level,
      regular_capacity: DEFAULT_CAPACITY,
      levels: PRIORITY_LEVELS,
    }
  }

  /// Sets the number of priority levels (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `levels` - Number of priority levels to set (minimum value is 1)
  ///
  /// # Returns
  ///
  /// Factory instance with updated settings
  pub fn with_levels(mut self, levels: usize) -> Self {
    self.levels = levels.max(1);
    self
  }

  /// Sets the regular queue capacity (builder pattern)
  ///
  /// # Arguments
  ///
  /// * `capacity` - Capacity of the regular message queue
  ///
  /// # Returns
  ///
  /// Factory instance with updated settings
  pub fn with_regular_capacity(mut self, capacity: usize) -> Self {
    self.regular_capacity = capacity;
    self
  }

  /// Creates a pair of mailbox and sender handle
  ///
  /// # Arguments
  ///
  /// * `options` - Mailbox capacity options
  ///
  /// # Returns
  ///
  /// `(TokioPriorityMailbox<M>, TokioPriorityMailboxSender<M>)` - Tuple of mailbox and sender handle
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (TokioPriorityMailbox<M>, TokioPriorityMailboxSender<M>)
  where
    M: Element, {
    let control_per_level = self.resolve_control_capacity(options.priority_capacity);
    let regular_capacity = self.resolve_regular_capacity(options.capacity);
    let queue = TokioPriorityQueues::<M>::new(self.levels, control_per_level, regular_capacity);
    let signal = NotifySignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (
      TokioPriorityMailbox { inner: mailbox },
      TokioPriorityMailboxSender { inner: sender },
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

impl<M> TokioPriorityMailbox<M>
where
  M: Element,
{
  /// Creates a new priority mailbox
  ///
  /// # Arguments
  ///
  /// * `control_capacity_per_level` - Capacity per priority level for the control queue
  ///
  /// # Returns
  ///
  /// `(TokioPriorityMailbox<M>, TokioPriorityMailboxSender<M>)` - Tuple of mailbox and sender handle
  pub fn new(control_capacity_per_level: usize) -> (Self, TokioPriorityMailboxSender<M>) {
    TokioPriorityMailboxRuntime::new(control_capacity_per_level).mailbox::<M>(MailboxOptions::default())
  }

  /// Returns a reference to the internal `QueueMailbox`
  ///
  /// # Returns
  ///
  /// An immutable reference to the internal mailbox
  pub fn inner(&self) -> &QueueMailbox<TokioPriorityQueues<M>, NotifySignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> Mailbox<PriorityEnvelope<M>> for TokioPriorityMailbox<M>
where
  M: Element,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, TokioPriorityQueues<M>, NotifySignal, PriorityEnvelope<M>>
  where
    Self: 'a;
  type SendError = PriorityQueueError<M>;

  fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), Self::SendError> {
    self.inner.try_send(message).map_err(Box::new)
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

impl<M> TokioPriorityMailboxSender<M>
where
  M: Element,
{
  /// Sends a message in a non-blocking manner
  ///
  /// # Arguments
  ///
  /// * `message` - The priority envelope to send
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully queued, `Err` if the queue is full
  ///
  /// # Errors
  ///
  /// Returns an error if the queue is full or sending fails
  pub fn try_send(&self, message: PriorityEnvelope<M>) -> Result<(), PriorityQueueError<M>> {
    self.inner.try_send(message).map_err(Box::new)
  }

  /// Sends a message asynchronously
  ///
  /// Waits until space becomes available in the queue.
  ///
  /// # Arguments
  ///
  /// * `message` - The priority envelope to send
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully sent, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if sending fails
  pub async fn send(&self, message: PriorityEnvelope<M>) -> Result<(), PriorityQueueError<M>> {
    self.inner.send(message).await.map_err(Box::new)
  }

  /// Sends a message with specified priority in a non-blocking manner
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully queued, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if the queue is full or sending fails
  pub fn try_send_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.try_send(PriorityEnvelope::new(message, priority))
  }

  /// Sends a message with specified priority asynchronously
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully sent, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if sending fails
  pub async fn send_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.send(PriorityEnvelope::new(message, priority)).await
  }

  /// Sends a control message with priority in a non-blocking manner
  ///
  /// Control messages are processed with higher priority than regular messages.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully queued, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if the queue is full or sending fails
  pub fn try_send_control_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.try_send(PriorityEnvelope::control(message, priority))
  }

  /// Sends a control message with priority asynchronously
  ///
  /// Control messages are processed with higher priority than regular messages.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  /// * `priority` - The priority of the message
  ///
  /// # Returns
  ///
  /// `Ok(())` if the message is successfully sent, `Err` on failure
  ///
  /// # Errors
  ///
  /// Returns an error if sending fails
  pub async fn send_control_with_priority(&self, message: M, priority: i8) -> Result<(), PriorityQueueError<M>> {
    self.send(PriorityEnvelope::control(message, priority)).await
  }

  /// Returns a reference to the internal `QueueMailboxProducer`
  ///
  /// # Returns
  ///
  /// An immutable reference to the internal producer
  pub fn inner(&self) -> &QueueMailboxProducer<TokioPriorityQueues<M>, NotifySignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}



