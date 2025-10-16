use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use cellex_utils_core_rs::Flag;
use cellex_utils_core_rs::{Element, QueueError, QueueRw, QueueSize};

use crate::internal::scheduler::ReadyQueueHandle;

use crate::internal::metrics::{MetricsEvent, MetricsSinkShared};

use super::mailbox_runtime::{Mailbox, MailboxSignal};

/// Mailbox implementation backed by a generic queue and notification signal.
///
/// Mailbox implementation based on generic queue and notification signal.
/// Designed to be runtime-agnostic without depending on specific async runtimes.
///
/// # Type Parameters
/// - `Q`: Message queue implementation type
/// - `S`: Notification signal implementation type
pub struct QueueMailbox<Q, S> {
  queue: Q,
  signal: S,
  closed: Flag,
  metrics_sink: Option<MetricsSinkShared>,
  scheduler_hook: Option<ReadyQueueHandle>,
}

impl<Q, S> QueueMailbox<Q, S> {
  /// Creates a new queue mailbox.
  ///
  /// # Arguments
  /// - `queue`: Message queue implementation
  /// - `signal`: Notification signal implementation
  pub fn new(queue: Q, signal: S) -> Self {
    Self {
      queue,
      signal,
      closed: Flag::default(),
      metrics_sink: None,
      scheduler_hook: None,
    }
  }

  /// Gets a reference to the internal queue.
  pub fn queue(&self) -> &Q {
    &self.queue
  }

  /// Gets a reference to the internal signal.
  pub fn signal(&self) -> &S {
    &self.signal
  }

  /// Creates a producer handle for sending messages.
  ///
  /// The producer can be shared across multiple threads and is used for sending messages to the mailbox.
  pub fn producer(&self) -> QueueMailboxProducer<Q, S>
  where
    Q: Clone,
    S: Clone, {
    QueueMailboxProducer {
      queue: self.queue.clone(),
      signal: self.signal.clone(),
      closed: self.closed.clone(),
      metrics_sink: self.metrics_sink.clone(),
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

  fn record_enqueue(&self) {
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
      queue: self.queue.clone(),
      signal: self.signal.clone(),
      closed: self.closed.clone(),
      metrics_sink: self.metrics_sink.clone(),
      scheduler_hook: self.scheduler_hook.clone(),
    }
  }
}

impl<Q, S> core::fmt::Debug for QueueMailbox<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailbox").finish()
  }
}

/// Sending handle that shares queue ownership with [`QueueMailbox`].
///
/// Sending handle that shares queue ownership with the mailbox.
/// Allows safe message sending from multiple threads.
///
/// # Type Parameters
/// - `Q`: Message queue implementation type
/// - `S`: Notification signal implementation type
#[derive(Clone)]
pub struct QueueMailboxProducer<Q, S> {
  queue: Q,
  signal: S,
  closed: Flag,
  metrics_sink: Option<MetricsSinkShared>,
  scheduler_hook: Option<ReadyQueueHandle>,
}

impl<Q, S> core::fmt::Debug for QueueMailboxProducer<Q, S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("QueueMailboxProducer").finish()
  }
}

unsafe impl<Q, S> Send for QueueMailboxProducer<Q, S>
where
  Q: Send + Sync,
  S: Send + Sync,
{
}

unsafe impl<Q, S> Sync for QueueMailboxProducer<Q, S>
where
  Q: Send + Sync,
  S: Send + Sync,
{
}

impl<Q, S> QueueMailboxProducer<Q, S> {
  /// Attempts to send a message (non-blocking).
  ///
  /// Returns an error immediately if the queue is full.
  ///
  /// # Arguments
  /// - `message`: Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, `Err(QueueError)` on failure
  ///
  /// # Errors
  /// - `QueueError::Disconnected`: Mailbox is closed
  /// - `QueueError::Full`: Queue is full
  pub fn try_send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    if self.closed.get() {
      return Err(QueueError::Disconnected);
    }

    match self.queue.offer(message) {
      Ok(()) => {
        self.signal.notify();
        if let Some(sink) = &self.metrics_sink {
          sink.with_ref(|sink| sink.record(MetricsEvent::MailboxEnqueued));
        }
        if let Some(hook) = &self.scheduler_hook {
          hook.notify_ready();
        }
        Ok(())
      }
      Err(err @ QueueError::Disconnected) | Err(err @ QueueError::Closed(_)) => {
        self.closed.set(true);
        Err(err)
      }
      Err(err) => Err(err),
    }
  }

  /// Sends a message using the mailbox queue.
  ///
  /// # Arguments
  /// - `message`: Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, `Err(QueueError)` on failure
  pub fn send<M>(&self, message: M) -> Result<(), QueueError<M>>
  where
    Q: QueueRw<M>,
    S: MailboxSignal,
    M: Element, {
    self.try_send(message)
  }

  /// Assigns a metrics sink for enqueue instrumentation.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }

  /// Installs a scheduler hook for notifying ready queue updates.
  pub fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    self.scheduler_hook = hook;
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
      Ok(()) => {
        self.signal.notify();
        self.record_enqueue();
        if let Some(hook) = &self.scheduler_hook {
          hook.notify_ready();
        }
        Ok(())
      }
      Err(err @ QueueError::Disconnected) | Err(err @ QueueError::Closed(_)) => {
        self.closed.set(true);
        Err(err)
      }
      Err(err) => Err(err),
    }
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    QueueMailboxRecv {
      mailbox: self,
      wait: None,
      marker: PhantomData,
    }
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

/// Future for receiving messages.
///
/// Future implementation for asynchronously receiving messages from the mailbox.
/// Waits until a message arrives and returns the arrived message.
///
/// # Type Parameters
/// - `'a`: Lifetime of the reference to the mailbox
/// - `Q`: Message queue implementation type
/// - `S`: Notification signal implementation type
/// - `M`: Type of the message to receive
pub struct QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element, {
  mailbox: &'a QueueMailbox<Q, S>,
  wait: Option<S::WaitFuture<'a>>,
  marker: PhantomData<M>,
}

impl<'a, Q, S, M> Future for QueueMailboxRecv<'a, Q, S, M>
where
  Q: QueueRw<M>,
  S: MailboxSignal,
  M: Element,
{
  type Output = Result<M, QueueError<M>>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = unsafe { self.get_unchecked_mut() };
    if this.mailbox.closed.get() {
      return Poll::Ready(Err(QueueError::Disconnected));
    }
    loop {
      match this.mailbox.queue.poll() {
        Ok(Some(message)) => {
          this.wait = None;
          return Poll::Ready(Ok(message));
        }
        Ok(None) => {
          if this.wait.is_none() {
            this.wait = Some(this.mailbox.signal.wait());
          }
        }
        Err(QueueError::Disconnected) => {
          this.mailbox.closed.set(true);
          this.wait = None;
          return Poll::Ready(Err(QueueError::Disconnected));
        }
        Err(QueueError::Closed(message)) => {
          this.mailbox.closed.set(true);
          this.wait = None;
          return Poll::Ready(Ok(message));
        }
        Err(QueueError::Full(_)) | Err(QueueError::OfferError(_)) => {
          return Poll::Pending;
        }
      }

      if let Some(wait) = this.wait.as_mut() {
        match unsafe { Pin::new_unchecked(wait) }.poll(cx) {
          Poll::Ready(()) => {
            this.wait = None;
            continue;
          }
          Poll::Pending => return Poll::Pending,
        }
      }
    }
  }
}
