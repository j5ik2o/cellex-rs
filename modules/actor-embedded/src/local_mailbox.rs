#[cfg(test)]
mod tests;

use alloc::rc::Rc;
use core::cell::RefCell;
use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use cellex_actor_core_rs::api::mailbox::MailboxOptions;
use cellex_actor_core_rs::api::mailbox::MailboxRuntime;
use cellex_actor_core_rs::api::mailbox::MailboxSignal;
use cellex_actor_core_rs::api::mailbox::QueueMailboxProducer;
#[cfg(feature = "embedded_rc")]
use cellex_actor_core_rs::api::mailbox::SingleThread;
#[cfg(not(feature = "embedded_rc"))]
use cellex_actor_core_rs::api::mailbox::ThreadSafe;
use cellex_actor_core_rs::api::mailbox::{Mailbox, MailboxPair};
use cellex_actor_core_rs::api::mailbox::{QueueMailbox, QueueMailboxRecv};
use cellex_actor_core_rs::internal::metrics::MetricsSinkShared;
#[cfg(not(feature = "embedded_rc"))]
use cellex_utils_embedded_rs::ArcLocalMpscUnboundedQueue;
#[cfg(feature = "embedded_rc")]
use cellex_utils_embedded_rs::RcMpscUnboundedQueue;
use cellex_utils_embedded_rs::{Element, QueueBase, QueueError, QueueRw, QueueSize};

#[cfg(feature = "embedded_rc")]
type LocalQueueInner<M> = Rc<RcMpscUnboundedQueue<M>>;

#[cfg(not(feature = "embedded_rc"))]
type LocalQueueInner<M> = ArcLocalMpscUnboundedQueue<M>;

#[cfg(feature = "embedded_rc")]
fn new_queue<M>() -> LocalQueueInner<M>
where
  M: Element, {
  Rc::new(RcMpscUnboundedQueue::new())
}

#[cfg(not(feature = "embedded_rc"))]
fn new_queue<M>() -> LocalQueueInner<M>
where
  M: Element, {
  ArcLocalMpscUnboundedQueue::new()
}

#[cfg(feature = "embedded_rc")]
fn clone_queue<M>(inner: &LocalQueueInner<M>) -> LocalQueueInner<M>
where
  M: Element, {
  Rc::clone(inner)
}

#[cfg(not(feature = "embedded_rc"))]
fn clone_queue<M>(inner: &LocalQueueInner<M>) -> LocalQueueInner<M>
where
  M: Element, {
  inner.clone()
}

#[derive(Debug)]
pub struct LocalQueue<M>
where
  M: Element, {
  inner: LocalQueueInner<M>,
}

impl<M> LocalQueue<M>
where
  M: Element,
{
  fn new() -> Self {
    Self { inner: new_queue() }
  }

  #[cfg(feature = "embedded_rc")]
  fn as_ref(&self) -> &RcMpscUnboundedQueue<M> {
    &self.inner
  }

  #[cfg(not(feature = "embedded_rc"))]
  fn as_ref(&self) -> &ArcLocalMpscUnboundedQueue<M> {
    &self.inner
  }
}

impl<M> Clone for LocalQueue<M>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self {
      inner: clone_queue(&self.inner),
    }
  }
}

impl<M> QueueBase<M> for LocalQueue<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    self.as_ref().len()
  }

  fn capacity(&self) -> QueueSize {
    self.as_ref().capacity()
  }
}

impl<M> QueueRw<M> for LocalQueue<M>
where
  M: Element,
{
  fn offer(&self, element: M) -> Result<(), QueueError<M>> {
    self.as_ref().offer(element)
  }

  fn poll(&self) -> Result<Option<M>, QueueError<M>> {
    self.as_ref().poll()
  }

  fn clean_up(&self) {
    self.as_ref().clean_up();
  }
}

/// Asynchronous mailbox for local thread.
///
/// Uses `Rc`-based queue to deliver messages in `!Send` environments.
pub struct LocalMailbox<M>
where
  M: Element, {
  inner: QueueMailbox<LocalQueue<M>, LocalSignal>,
}

/// Message sender to `LocalMailbox`.
///
/// A handle for sending messages to the mailbox asynchronously.
pub struct LocalMailboxSender<M>
where
  M: Element, {
  inner: QueueMailboxProducer<LocalQueue<M>, LocalSignal>,
}

/// Factory that creates local actor runtime.
///
/// Creates mailbox pairs for embedded or single-threaded environments.
#[derive(Clone, Debug, Default)]
pub struct LocalMailboxRuntime {
  _marker: PhantomData<()>,
}

#[derive(Clone, Debug, Default)]
pub struct LocalSignal {
  state: Rc<RefCell<SignalState>>,
}

#[derive(Debug, Default)]
struct SignalState {
  notified: bool,
  waker: Option<Waker>,
}

impl MailboxSignal for LocalSignal {
  type WaitFuture<'a>
    = LocalSignalWait
  where
    Self: 'a;

  fn notify(&self) {
    let mut state = self.state.borrow_mut();
    state.notified = true;
    if let Some(waker) = state.waker.take() {
      waker.wake();
    }
  }

  fn wait(&self) -> Self::WaitFuture<'_> {
    LocalSignalWait { signal: self.clone() }
  }
}

pub struct LocalSignalWait {
  signal: LocalSignal,
}

impl Future for LocalSignalWait {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut state = self.signal.state.borrow_mut();
    if state.notified {
      state.notified = false;
      Poll::Ready(())
    } else {
      state.waker = Some(cx.waker().clone());
      Poll::Pending
    }
  }
}

impl LocalMailboxRuntime {
  /// Creates a new `LocalMailboxRuntime`.
  ///
  /// # Returns
  ///
  /// A new factory instance
  pub const fn new() -> Self {
    Self { _marker: PhantomData }
  }

  /// Creates a mailbox pair with the specified options.
  ///
  /// # Arguments
  ///
  /// * `options` - Mailbox configuration options
  ///
  /// # Returns
  ///
  /// A tuple of (receiver mailbox, sender handle)
  pub fn mailbox<M>(&self, options: MailboxOptions) -> (LocalMailbox<M>, LocalMailboxSender<M>)
  where
    M: Element, {
    let (mailbox, sender) = self.build_mailbox::<M>(options);
    (LocalMailbox { inner: mailbox }, LocalMailboxSender { inner: sender })
  }

  /// Creates an unbounded mailbox pair.
  ///
  /// # Returns
  ///
  /// A tuple of (receiver mailbox, sender handle)
  pub fn unbounded<M>(&self) -> (LocalMailbox<M>, LocalMailboxSender<M>)
  where
    M: Element, {
    self.mailbox(MailboxOptions::unbounded())
  }
}

impl MailboxRuntime for LocalMailboxRuntime {
  #[cfg(feature = "embedded_rc")]
  type Concurrency = SingleThread;
  #[cfg(not(feature = "embedded_rc"))]
  type Concurrency = ThreadSafe;
  type Mailbox<M>
    = QueueMailbox<Self::Queue<M>, Self::Signal>
  where
    M: Element;
  type Producer<M>
    = QueueMailboxProducer<Self::Queue<M>, Self::Signal>
  where
    M: Element;
  type Queue<M>
    = LocalQueue<M>
  where
    M: Element;
  type Signal = LocalSignal;

  fn build_mailbox<M>(&self, _options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let queue = LocalQueue::new();
    let signal = LocalSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}

impl<M> LocalMailbox<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  /// Creates a new mailbox pair with default settings.
  ///
  /// # Returns
  ///
  /// A tuple of (receiver mailbox, sender handle)
  pub fn new() -> (Self, LocalMailboxSender<M>) {
    LocalMailboxRuntime::default().unbounded()
  }

  /// Creates a new sender handle.
  ///
  /// # Returns
  ///
  /// A new sender to the mailbox
  pub fn producer(&self) -> LocalMailboxSender<M>
  where
    LocalSignal: Clone, {
    LocalMailboxSender {
      inner: self.inner.producer(),
    }
  }

  /// Returns a reference to the internal queue mailbox.
  ///
  /// # Returns
  ///
  /// A reference to the `QueueMailbox`
  pub fn inner(&self) -> &QueueMailbox<LocalQueue<M>, LocalSignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> Mailbox<M> for LocalMailbox<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, LocalQueue<M>, LocalSignal, M>
  where
    Self: 'a;
  type SendError = QueueError<M>;

  fn try_send(&self, message: M) -> Result<(), Self::SendError> {
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

impl<M> Clone for LocalMailbox<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<M> fmt::Debug for LocalMailbox<M>
where
  M: Element,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LocalMailbox").finish()
  }
}

impl<M> LocalMailboxSender<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  /// Sends a message immediately (non-blocking).
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  ///
  /// # Errors
  ///
  /// Returns `QueueError` if the queue is full or closed
  pub fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.try_send(message)
  }

  /// Sends a message to the mailbox.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to send
  ///
  /// # Errors
  ///
  /// Returns `QueueError` if the queue is closed
  pub fn send(&self, message: M) -> Result<(), QueueError<M>> {
    self.inner.send(message)
  }

  /// Returns a reference to the internal mailbox producer.
  ///
  /// # Returns
  ///
  /// A reference to the `QueueMailboxProducer`
  pub fn inner(&self) -> &QueueMailboxProducer<LocalQueue<M>, LocalSignal> {
    &self.inner
  }

  /// Assigns a metrics sink to the underlying producer.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M> Clone for LocalMailboxSender<M>
where
  M: Element,
  LocalQueue<M>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<M> fmt::Debug for LocalMailboxSender<M>
where
  M: Element,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LocalMailboxSender").finish()
  }
}
