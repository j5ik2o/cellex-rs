use std::fmt;

use cellex_utils_core_rs::{
  sync::ArcShared,
  v2::collections::{
    queue::{
      backend::{AsyncQueueBackend, OfferOutcome, QueueError},
      capabilities::MultiProducer,
      type_keys::MpscKey,
      AsyncQueue,
    },
    wait::{WaitHandle, WaitQueue},
  },
};
use tokio::sync::mpsc;

use crate::v2::TokioAsyncMutex;

/// Bounded MPSC queue backend backed by Tokio's [`mpsc`] channel.
pub struct TokioBoundedMpscBackend<T> {
  sender:           Option<mpsc::Sender<T>>,
  receiver:         mpsc::Receiver<T>,
  capacity:         usize,
  closed:           bool,
  producer_waiters: WaitQueue<QueueError<T>>,
  consumer_waiters: WaitQueue<QueueError<T>>,
}

impl<T> TokioBoundedMpscBackend<T> {
  /// Creates a backend instance backed by a Tokio channel with the given capacity.
  #[must_use]
  pub fn new(capacity: usize) -> Self {
    assert!(capacity > 0, "TokioBoundedMpscBackend requires capacity > 0");
    let (sender, receiver) = mpsc::channel(capacity);
    Self {
      sender: Some(sender),
      receiver,
      capacity,
      closed: false,
      producer_waiters: WaitQueue::<QueueError<T>>::new(),
      consumer_waiters: WaitQueue::<QueueError<T>>::new(),
    }
  }

  fn register_producer_waiter(&mut self) -> WaitHandle<QueueError<T>> {
    self.producer_waiters.register()
  }

  fn register_consumer_waiter(&mut self) -> WaitHandle<QueueError<T>> {
    self.consumer_waiters.register()
  }

  fn notify_producer_waiter(&mut self) {
    let _ = self.producer_waiters.notify_success();
  }

  fn notify_consumer_waiter(&mut self) {
    let _ = self.consumer_waiters.notify_success();
  }

  fn fail_all_waiters<F>(&mut self, mut make_error: F)
  where
    F: FnMut() -> QueueError<T>, {
    self.producer_waiters.notify_error_all_with(|| make_error());
    self.consumer_waiters.notify_error_all_with(|| make_error());
  }

  fn mark_closed(&mut self) {
    self.closed = true;
    self.sender = None;
    self.receiver.close();
    self.fail_all_waiters(|| QueueError::Disconnected);
  }
}

impl<T> fmt::Debug for TokioBoundedMpscBackend<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("TokioBoundedMpscBackend")
      .field("capacity", &self.capacity)
      .field("len", &self.receiver.len())
      .field("closed", &self.closed)
      .finish()
  }
}

#[async_trait::async_trait(?Send)]
impl<T> AsyncQueueBackend<T> for TokioBoundedMpscBackend<T>
where
  T: Send + 'static,
{
  async fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError<T>> {
    if self.closed {
      return Err(QueueError::Closed(item));
    }

    let sender = match self.sender.as_ref() {
      | Some(sender) => sender,
      | None => return Err(QueueError::Closed(item)),
    };

    match sender.send(item).await {
      | Ok(()) => {
        self.notify_consumer_waiter();
        Ok(OfferOutcome::Enqueued)
      },
      | Err(error) => {
        let value = error.0;
        self.mark_closed();
        Err(QueueError::Closed(value))
      },
    }
  }

  async fn poll(&mut self) -> Result<T, QueueError<T>> {
    match self.receiver.recv().await {
      | Some(item) => {
        self.notify_producer_waiter();
        Ok(item)
      },
      | None => {
        self.mark_closed();
        Err(QueueError::Disconnected)
      },
    }
  }

  async fn close(&mut self) -> Result<(), QueueError<T>> {
    self.mark_closed();
    Ok(())
  }

  fn len(&self) -> usize {
    self.receiver.len()
  }

  fn capacity(&self) -> usize {
    self.capacity
  }

  fn prepare_producer_wait(&mut self) -> Option<WaitHandle<QueueError<T>>> {
    if self.closed || self.sender.is_none() {
      None
    } else {
      Some(self.register_producer_waiter())
    }
  }

  fn prepare_consumer_wait(&mut self) -> Option<WaitHandle<QueueError<T>>> {
    if self.closed || self.sender.is_none() {
      None
    } else {
      Some(self.register_consumer_waiter())
    }
  }

  fn is_closed(&self) -> bool {
    self.closed || self.sender.as_ref().map_or(true, tokio::sync::mpsc::Sender::is_closed)
  }
}

/// Tokio-based async MPSC queue alias using [`TokioBoundedMpscBackend`].
pub type TokioMpscQueue<T> =
  AsyncQueue<T, MpscKey, TokioBoundedMpscBackend<T>, TokioAsyncMutex<TokioBoundedMpscBackend<T>>>;

/// Constructs an [`AsyncQueue`] configured with the Tokio bounded MPSC backend and mutex wrapper.
pub fn make_tokio_mpsc_queue<T>(capacity: usize) -> TokioMpscQueue<T>
where
  T: Send + 'static,
  MpscKey: MultiProducer, {
  let backend = TokioBoundedMpscBackend::new(capacity);
  let shared = ArcShared::new(TokioAsyncMutex::new(backend));
  AsyncQueue::<T, MpscKey, _, _>::new_mpsc(shared)
}
