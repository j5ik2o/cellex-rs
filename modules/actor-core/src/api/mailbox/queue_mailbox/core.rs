use cellex_utils_core_rs::{collections::queue::QueueError, Element, Flag, QueueRw, QueueSize};

use crate::api::{
  actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
  mailbox::MailboxSignal,
  metrics::{MetricsEvent, MetricsSinkShared},
};

/// Core mailbox state shared between handle and producer implementations.
pub struct MailboxQueueCore<Q, S> {
  queue:          Q,
  signal:         S,
  closed:         Flag,
  metrics_sink:   Option<MetricsSinkShared>,
  scheduler_hook: Option<ReadyQueueHandle>,
}

impl<Q, S> MailboxQueueCore<Q, S> {
  /// Creates a new core with the provided queue and signal.
  #[must_use]
  pub fn new(queue: Q, signal: S) -> Self {
    Self { queue, signal, closed: Flag::default(), metrics_sink: None, scheduler_hook: None }
  }

  /// Returns a reference to the underlying queue.
  #[must_use]
  pub const fn queue(&self) -> &Q {
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
    Q: QueueRw<M>,
    M: Element, {
    self.queue.len()
  }

  /// Returns the current queue capacity.
  #[must_use]
  pub fn capacity<M>(&self) -> QueueSize
  where
    Q: QueueRw<M>,
    M: Element, {
    self.queue.capacity()
  }

  /// Attempts to enqueue a message into the underlying queue.
  pub fn try_send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    if self.closed.get() {
      return Err(QueueError::Disconnected);
    }

    match self.queue.offer(message) {
      | Ok(()) => {
        self.signal.notify();
        self.record_enqueue();
        self.notify_ready();
        Ok(())
      },
      | Err(err @ QueueError::Disconnected) | Err(err @ QueueError::Closed(_)) => {
        self.closed.set(true);
        Err(err)
      },
      | Err(err) => Err(err),
    }
  }

  /// Attempts to dequeue a message from the underlying queue.
  pub fn try_dequeue<M>(&self) -> Result<Option<M>, QueueError<M>>
  where
    Q: QueueRw<M>,
    M: Element, {
    self.queue.poll()
  }

  /// Closes the queue and notifies waiting receivers.
  pub fn close<M>(&self)
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.queue.clean_up();
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
    if let Some(sink) = &self.metrics_sink {
      sink.with_ref(|sink| sink.record(MetricsEvent::MailboxEnqueued));
    }
  }
}

impl<Q, S> Clone for MailboxQueueCore<Q, S>
where
  Q: Clone,
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

impl<Q, S> core::fmt::Debug for MailboxQueueCore<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("MailboxQueueCore").finish()
  }
}
