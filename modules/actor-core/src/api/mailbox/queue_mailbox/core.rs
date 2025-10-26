use cellex_utils_core_rs::{
  collections::{queue::QueueSize, Element},
  sync::Flag,
  v2::collections::queue::backend::{OfferOutcome, QueueError},
};

use super::{MailboxQueueDriver, QueuePollOutcome};
use crate::{
  api::{
    actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
    mailbox::MailboxError,
    metrics::{MetricsEvent, MetricsSinkShared},
  },
  shared::mailbox::MailboxSignal,
};

/// Core mailbox state shared between handle and producer implementations.
pub struct MailboxQueueCore<D, S> {
  queue:          D,
  signal:         S,
  closed:         Flag,
  metrics_sink:   Option<MetricsSinkShared>,
  scheduler_hook: Option<ReadyQueueHandle>,
}

impl<D, S> MailboxQueueCore<D, S> {
  /// Creates a new core with the provided queue and signal.
  #[must_use]
  pub fn new(queue: D, signal: S) -> Self {
    Self { queue, signal, closed: Flag::default(), metrics_sink: None, scheduler_hook: None }
  }

  /// Returns a reference to the underlying queue driver.
  #[must_use]
  pub const fn queue(&self) -> &D {
    &self.queue
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

  /// Returns the current queue length.
  #[must_use]
  pub fn len<M>(&self) -> QueueSize
  where
    D: MailboxQueueDriver<M>,
    M: Element, {
    self.queue.len()
  }

  /// Returns the current queue capacity.
  #[must_use]
  pub fn capacity<M>(&self) -> QueueSize
  where
    D: MailboxQueueDriver<M>,
    M: Element, {
    self.queue.capacity()
  }

  /// Attempts to enqueue a message and returns mailbox-level errors.
  pub fn try_send_mailbox<M>(&self, message: M) -> Result<OfferOutcome, MailboxError<M>>
  where
    D: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    if self.closed.get() {
      return Err(MailboxError::Disconnected);
    }

    match self.queue.offer(message) {
      | Ok(outcome) => {
        self.signal.notify();
        self.record_enqueue();
        self.notify_ready();
        match outcome {
          | OfferOutcome::Enqueued | OfferOutcome::DroppedOldest { .. } | OfferOutcome::GrewTo { .. } => Ok(outcome),
          | OfferOutcome::DroppedNewest { .. } => {
            panic!("MailboxQueueDriver must map DroppedNewest into an error before returning success");
          },
        }
      },
      | Err(error) => {
        let mailbox_error = self.convert_queue_error(error);
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
    D: MailboxQueueDriver<M>,
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
    D: MailboxQueueDriver<M>,
    M: Element, {
    match self.queue.poll() {
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
    D: MailboxQueueDriver<M>,
    M: Element, {
    match self.try_dequeue_mailbox() {
      | Ok(value) => Ok(value),
      | Err(error) => Err(error.into()),
    }
  }

  /// Closes the queue and notifies waiting receivers.
  pub fn close<M>(&self)
  where
    D: MailboxQueueDriver<M>,
    S: MailboxSignal,
    M: Element, {
    let _ = self.queue.close();
    self.signal.notify();
    self.closed.set(true);
  }

  /// Returns whether the queue has been closed.
  #[must_use]
  pub fn is_closed(&self) -> bool {
    self.closed.get()
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

  fn handle_queue_error<M>(&self, error: QueueError<M>) -> Result<Option<M>, MailboxError<M>>
  where
    D: MailboxQueueDriver<M>,
    M: Element, {
    let mailbox_error = self.convert_queue_error(error);
    if mailbox_error.closes_mailbox() {
      self.closed.set(true);
    }
    Err(mailbox_error)
  }

  fn convert_queue_error<M>(&self, error: QueueError<M>) -> MailboxError<M>
  where
    D: MailboxQueueDriver<M>,
    M: Element, {
    match error {
      | QueueError::Full(message) => match self.queue.overflow_policy() {
        | Some(policy) => MailboxError::from_queue_error_with_policy(QueueError::Full(message), policy),
        | None => MailboxError::from_queue_error(QueueError::Full(message)),
      },
      | other => MailboxError::from_queue_error(other),
    }
  }
}

impl<D, S> Clone for MailboxQueueCore<D, S>
where
  D: Clone,
  S: Clone,
{
  fn clone(&self) -> Self {
    Self {
      queue:          self.queue.clone(),
      signal:         self.signal.clone(),
      closed:         self.closed.clone(),
      metrics_sink:   self.metrics_sink.clone(),
      scheduler_hook: self.scheduler_hook.clone(),
    }
  }
}

impl<D, S> core::fmt::Debug for MailboxQueueCore<D, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("MailboxQueueCore").finish()
  }
}
