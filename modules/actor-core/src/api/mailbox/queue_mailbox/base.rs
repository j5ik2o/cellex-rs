use cellex_utils_core_rs::{collections::queue::QueueError, Element, QueueRw, QueueSize};

use super::recv::QueueMailboxRecv;
use crate::api::{
  actor_scheduler::ready_queue_scheduler::ReadyQueueHandle,
  mailbox::{queue_mailbox_producer::QueueMailboxProducer, Mailbox, MailboxHandle, MailboxProducer, MailboxSignal},
  metrics::{MetricsEvent, MetricsSinkShared},
};

/// Mailbox implementation backed by a generic queue and notification signal.
pub struct QueueMailbox<Q, S> {
  pub(super) queue:          Q,
  pub(super) signal:         S,
  pub(super) closed:         cellex_utils_core_rs::Flag,
  pub(super) metrics_sink:   Option<MetricsSinkShared>,
  pub(super) scheduler_hook: Option<ReadyQueueHandle>,
}

impl<Q, S> QueueMailbox<Q, S> {
  /// Creates a new queue mailbox.
  pub fn new(queue: Q, signal: S) -> Self {
    Self { queue, signal, closed: cellex_utils_core_rs::Flag::default(), metrics_sink: None, scheduler_hook: None }
  }

  /// Gets a reference to the internal queue.
  #[must_use]
  pub const fn queue(&self) -> &Q {
    &self.queue
  }

  /// Gets a reference to the internal signal.
  #[must_use]
  pub const fn signal(&self) -> &S {
    &self.signal
  }

  /// Creates a producer handle for sending messages.
  pub fn producer(&self) -> QueueMailboxProducer<Q, S>
  where
    Q: Clone,
    S: Clone, {
    QueueMailboxProducer {
      queue:          self.queue.clone(),
      signal:         self.signal.clone(),
      closed:         self.closed.clone(),
      metrics_sink:   self.metrics_sink.clone(),
      scheduler_hook: self.scheduler_hook.clone(),
    }
  }

  /// Configures a metrics sink used for enqueue instrumentation.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }

  /// Installs a scheduler hook that is notified when new messages arrive.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.scheduler_hook = hook;
  }

  /// Returns the current queue length as `usize`.
  #[must_use]
  pub fn len_usize<M>(&self) -> usize
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.queue.len().to_usize()
  }

  /// Returns the queue capacity as `usize`.
  #[must_use]
  pub fn capacity_usize<M>(&self) -> usize
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.queue.capacity().to_usize()
  }

  pub(super) fn record_enqueue(&self) {
    if let Some(sink) = &self.metrics_sink {
      sink.with_ref(|sink| sink.record(MetricsEvent::MailboxEnqueued));
    }
  }
}

impl<Q, S> Clone for QueueMailbox<Q, S>
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

impl<Q, S> core::fmt::Debug for QueueMailbox<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailbox").finish()
  }
}

impl<M, Q, S> MailboxHandle<M> for QueueMailbox<Q, S>
where
  Q: QueueRw<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  type Signal = S;

  fn signal(&self) -> Self::Signal {
    self.signal().clone()
  }

  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>> {
    self.queue().poll()
  }
}

impl<M, Q, S> MailboxProducer<M> for QueueMailboxProducer<Q, S>
where
  Q: QueueRw<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    <QueueMailboxProducer<Q, S>>::try_send(self, message)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    <QueueMailboxProducer<Q, S>>::set_metrics_sink(self, sink);
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    <QueueMailboxProducer<Q, S>>::set_scheduler_hook(self, hook);
  }
}

impl<M, Q, S> Mailbox<M> for QueueMailbox<Q, S>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, Q, S, M>
  where
    Self: 'a;
  type SendError = QueueError<M>;

  fn try_send(&self, message: M) -> Result<(), Self::SendError> {
    match self.queue.offer(message) {
      | Ok(()) => {
        self.signal.notify();
        self.record_enqueue();
        if let Some(hook) = &self.scheduler_hook {
          hook.notify_ready();
        }
        Ok(())
      },
      | Err(err @ QueueError::Disconnected) | Err(err @ QueueError::Closed(_)) => {
        self.closed.set(true);
        Err(err)
      },
      | Err(err) => Err(err),
    }
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    QueueMailboxRecv::new(self)
  }

  fn len(&self) -> QueueSize {
    self.queue.len()
  }

  fn capacity(&self) -> QueueSize {
    self.queue.capacity()
  }

  fn close(&self) {
    self.queue.clean_up();
    self.signal.notify();
    self.closed.set(true);
  }

  fn is_closed(&self) -> bool {
    self.closed.get()
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.scheduler_hook = hook;
  }
}
