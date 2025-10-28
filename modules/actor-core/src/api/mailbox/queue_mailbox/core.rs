use cellex_utils_core_rs::{
  collections::{
    queue::{
      backend::{OfferOutcome, QueueError},
      QueueSize,
    },
    Element,
  },
  sync::Flag,
};

use super::{queue::MailboxQueue, QueuePollOutcome, SystemMailboxLane};
use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    mailbox::{MailboxError, MailboxOverflowPolicy},
    metrics::{MetricsEvent, MetricsSinkShared},
  },
  shared::mailbox::MailboxSignal,
};

fn sum_queue_size(first: QueueSize, second: QueueSize) -> QueueSize {
  if first.is_limitless() || second.is_limitless() {
    QueueSize::limitless()
  } else {
    QueueSize::limited(first.to_usize().saturating_add(second.to_usize()))
  }
}

/// Core mailbox state shared between handle and producer implementations.
pub struct QueueMailboxCore<SQ, UQ, S> {
  system_queue:   Option<SQ>,
  user_queue:     UQ,
  signal:         S,
  closed:         Flag,
  metrics_sink:   Option<MetricsSinkShared>,
  scheduler_hook: Option<ReadyQueueHandle>,
}

impl<SQ, UQ, S> QueueMailboxCore<SQ, UQ, S> {
  /// Creates a new core with the provided queues and signal.
  #[must_use]
  pub fn new(system_queue: Option<SQ>, user_queue: UQ, signal: S) -> Self {
    Self { system_queue, user_queue, signal, closed: Flag::default(), metrics_sink: None, scheduler_hook: None }
  }

  /// Returns a reference to the system queue if available.
  #[must_use]
  pub const fn system_queue(&self) -> Option<&SQ> {
    self.system_queue.as_ref()
  }

  /// Returns a reference to the user queue.
  #[must_use]
  pub const fn user_queue(&self) -> &UQ {
    &self.user_queue
  }

  /// Returns a reference to the notification signal.
  #[must_use]
  pub const fn signal(&self) -> &S {
    &self.signal
  }

  /// Returns the closed flag for this mailbox.
  #[must_use]
  pub const fn closed(&self) -> &Flag {
    &self.closed
  }

  /// Updates the metrics sink used for enqueue instrumentation.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }

  /// Updates the scheduler hook invoked on enqueue.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.scheduler_hook = hook;
  }

  /// Installs metrics sinks on both queues.
  pub fn apply_queue_metrics_sink<M>(&self, sink: Option<MetricsSinkShared>)
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    if let Some(system_queue) = &self.system_queue {
      system_queue.set_metrics_sink(sink.clone());
    }
    self.user_queue.set_metrics_sink(sink);
  }

  /// Returns the current queue length.
  #[must_use]
  pub fn len<M>(&self) -> QueueSize
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    let user_len = self.user_queue.len();
    let system_len = self.system_queue.as_ref().map(|queue| queue.len()).unwrap_or_else(|| QueueSize::limited(0));
    sum_queue_size(system_len, user_len)
  }

  /// Returns the current queue capacity.
  #[must_use]
  pub fn capacity<M>(&self) -> QueueSize
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    let user_cap = self.user_queue.capacity();
    let system_cap = self.system_queue.as_ref().map(|queue| queue.capacity()).unwrap_or_else(|| QueueSize::limited(0));
    sum_queue_size(system_cap, user_cap)
  }

  /// Attempts to enqueue a message and returns mailbox-level errors.
  pub fn try_send_mailbox<M>(&self, message: M) -> Result<OfferOutcome, MailboxError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    if self.closed.get() {
      return Err(MailboxError::Disconnected);
    }

    if let Some(system_queue) = &self.system_queue {
      if system_queue.accepts(&message) {
        match system_queue.offer(message) {
          | Ok(outcome) => return self.finish_enqueue::<M>(outcome),
          | Err(QueueError::Full(returned)) => {
            let mailbox_error = self.convert_user_queue_error(QueueError::Full(returned));
            if mailbox_error.closes_mailbox() {
              self.closed.set(true);
            }
            return Err(mailbox_error);
          },
          | Err(error) => {
            let mailbox_error = self.convert_user_queue_error(error);
            if mailbox_error.closes_mailbox() {
              self.closed.set(true);
            }
            return Err(mailbox_error);
          },
        }
      }
    }

    match self.user_queue.offer(message) {
      | Ok(outcome) => self.finish_enqueue::<M>(outcome),
      | Err(error) => {
        let mailbox_error = self.convert_user_queue_error(error);
        if mailbox_error.closes_mailbox() {
          self.closed.set(true);
        }
        Err(mailbox_error)
      },
    }
  }

  /// Attempts to enqueue a message into the underlying queue, returning legacy queue errors.
  pub fn try_send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    match self.try_send_mailbox(message) {
      | Ok(_) => Ok(()),
      | Err(error) => Err(error.into()),
    }
  }

  /// Attempts to dequeue a message, returning mailbox-level errors.
  pub fn try_dequeue_mailbox<M>(&self) -> Result<Option<M>, MailboxError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    if let Some(system_queue) = &self.system_queue {
      match system_queue.poll() {
        | Ok(QueuePollOutcome::Message(message)) => return Ok(Some(message)),
        | Ok(QueuePollOutcome::Empty) | Ok(QueuePollOutcome::Pending) => {},
        | Ok(QueuePollOutcome::Disconnected) => {
          self.closed.set(true);
          return Err(MailboxError::Disconnected);
        },
        | Ok(QueuePollOutcome::Closed(message)) => {
          self.closed.set(true);
          return Err(MailboxError::Closed { last: Some(message) });
        },
        | Ok(QueuePollOutcome::Err(error)) => return self.handle_queue_error(error),
        | Err(error) => return self.handle_queue_error(error),
      }
    }

    match self.user_queue.poll() {
      | Ok(QueuePollOutcome::Message(message)) => Ok(Some(message)),
      | Ok(QueuePollOutcome::Empty) | Ok(QueuePollOutcome::Pending) => Ok(None),
      | Ok(QueuePollOutcome::Disconnected) => {
        self.closed.set(true);
        Err(MailboxError::Disconnected)
      },
      | Ok(QueuePollOutcome::Closed(message)) => {
        self.closed.set(true);
        Err(MailboxError::Closed { last: Some(message) })
      },
      | Ok(QueuePollOutcome::Err(error)) => self.handle_queue_error(error),
      | Err(error) => self.handle_queue_error(error),
    }
  }

  /// Attempts to dequeue a message from the underlying queue, returning legacy queue errors.
  pub fn try_dequeue<M>(&self) -> Result<Option<M>, QueueError<M>>
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    M: Element, {
    match self.try_dequeue_mailbox() {
      | Ok(value) => Ok(value),
      | Err(error) => Err(error.into()),
    }
  }

  /// Closes the queue and notifies waiting receivers.
  pub fn close<M>(&self)
  where
    SQ: SystemMailboxLane<M>,
    UQ: MailboxQueue<M>,
    S: MailboxSignal,
    M: Element, {
    if let Some(system_queue) = &self.system_queue {
      let _ = system_queue.close();
    }
    let _ = self.user_queue.close();
    self.signal.notify();
    self.closed.set(true);
  }

  /// Returns whether the queue has been closed.
  #[must_use]
  pub fn is_closed(&self) -> bool {
    self.closed.get()
  }

  fn finish_enqueue<M>(&self, outcome: OfferOutcome) -> Result<OfferOutcome, MailboxError<M>>
  where
    S: MailboxSignal,
    M: Element, {
    self.signal.notify();
    self.record_enqueue();
    self.notify_ready();
    match outcome {
      | OfferOutcome::Enqueued | OfferOutcome::DroppedOldest { .. } | OfferOutcome::GrewTo { .. } => Ok(outcome),
      | OfferOutcome::DroppedNewest { .. } => {
        panic!("MailboxQueue implementors must map DroppedNewest into an error before returning success");
      },
    }
  }

  fn notify_ready(&self) {
    if let Some(hook) = &self.scheduler_hook {
      hook.notify_ready();
    }
  }

  fn record_enqueue(&self) {
    self.record_event(MetricsEvent::MailboxEnqueued);
  }

  fn record_event(&self, event: MetricsEvent) {
    if let Some(sink) = &self.metrics_sink {
      sink.with_ref(|sink| sink.record(event));
    }
  }

  fn convert_user_queue_error<M>(&self, error: QueueError<M>) -> MailboxError<M>
  where
    UQ: MailboxQueue<M>,
    M: Element, {
    let policy = self.user_queue.overflow_policy();
    Self::map_queue_error_with_policy(error, policy)
  }

  fn handle_queue_error<M>(&self, error: QueueError<M>) -> Result<Option<M>, MailboxError<M>>
  where
    UQ: MailboxQueue<M>,
    M: Element, {
    let mailbox_error = self.convert_user_queue_error(error);
    if mailbox_error.closes_mailbox() {
      self.closed.set(true);
    }
    Err(mailbox_error)
  }

  fn map_queue_error_with_policy<M>(error: QueueError<M>, policy: Option<MailboxOverflowPolicy>) -> MailboxError<M>
  where
    M: Element, {
    match (error, policy) {
      | (QueueError::Full(message), Some(policy)) => {
        MailboxError::from_queue_error_with_policy(QueueError::Full(message), policy)
      },
      | (other, _) => MailboxError::from_queue_error(other),
    }
  }
}

impl<SQ, UQ, S> Clone for QueueMailboxCore<SQ, UQ, S>
where
  SQ: Clone,
  UQ: Clone,
  S: Clone,
{
  fn clone(&self) -> Self {
    Self {
      system_queue:   self.system_queue.clone(),
      user_queue:     self.user_queue.clone(),
      signal:         self.signal.clone(),
      closed:         self.closed.clone(),
      metrics_sink:   self.metrics_sink.clone(),
      scheduler_hook: self.scheduler_hook.clone(),
    }
  }
}

impl<SQ, UQ, S> core::fmt::Debug for QueueMailboxCore<SQ, UQ, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailboxCore").finish()
  }
}
