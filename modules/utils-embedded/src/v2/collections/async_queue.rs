//! Async queue adapters for Embassy environments.

use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
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
use embassy_sync::{
  blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex},
  channel::{Channel, TryReceiveError, TrySendError},
};

use crate::v2::sync::EmbassyAsyncMutex;

#[cfg(all(feature = "embassy", test, not(target_os = "none")))]
mod tests;

/// Bounded MPSC backend backed by [`embassy_sync::channel::Channel`].
pub struct EmbassyBoundedMpscBackend<T, M, const N: usize>
where
  M: RawMutex, {
  channel:          Channel<M, T, N>,
  size:             AtomicUsize,
  closed:           AtomicBool,
  producer_waiters: WaitQueue<QueueError<T>>,
  consumer_waiters: WaitQueue<QueueError<T>>,
}

impl<T, M, const N: usize> EmbassyBoundedMpscBackend<T, M, N>
where
  M: RawMutex,
{
  /// Creates a new backend instance.
  pub const fn new() -> Self {
    Self {
      channel:          Channel::new(),
      size:             AtomicUsize::new(0),
      closed:           AtomicBool::new(false),
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
    self.closed.store(true, Ordering::SeqCst);
    self.fail_all_waiters(|| QueueError::Disconnected);
  }

  fn is_closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }

  fn len(&self) -> usize {
    self.size.load(Ordering::SeqCst)
  }

  fn capacity(&self) -> usize {
    N
  }
}

#[async_trait(?Send)]
impl<T, M, const N: usize> AsyncQueueBackend<T> for EmbassyBoundedMpscBackend<T, M, N>
where
  M: RawMutex,
  T: Send,
{
  async fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError<T>> {
    if self.is_closed() {
      return Err(QueueError::Closed(item));
    }

    let mut value = Some(item);

    loop {
      match self.channel.try_send(value.take().expect("value consumed")) {
        | Ok(()) => {
          self.size.fetch_add(1, Ordering::SeqCst);
          self.notify_consumer_waiter();
          return Ok(OfferOutcome::Enqueued);
        },
        | Err(TrySendError::Full(v)) => {
          value = Some(v);
          if self.is_closed() {
            return Err(QueueError::Closed(value.take().expect("value consumed")));
          }
          let waiter = self.register_producer_waiter();
          waiter.await?;
        },
      }
    }
  }

  async fn poll(&mut self) -> Result<T, QueueError<T>> {
    loop {
      match self.channel.try_receive() {
        | Ok(value) => {
          self.size.fetch_sub(1, Ordering::SeqCst);
          self.notify_producer_waiter();
          return Ok(value);
        },
        | Err(TryReceiveError::Empty) => {
          if self.is_closed() {
            return Err(QueueError::Disconnected);
          }
          let waiter = self.register_consumer_waiter();
          waiter.await?;
        },
      }
    }
  }

  async fn close(&mut self) -> Result<(), QueueError<T>> {
    self.mark_closed();
    while self.channel.try_receive().is_ok() {
      self.size.fetch_sub(1, Ordering::SeqCst);
    }
    Ok(())
  }

  fn len(&self) -> usize {
    self.len()
  }

  fn capacity(&self) -> usize {
    self.capacity()
  }

  fn prepare_producer_wait(&mut self) -> Option<WaitHandle<QueueError<T>>> {
    if self.is_closed() {
      None
    } else {
      Some(self.register_producer_waiter())
    }
  }

  fn prepare_consumer_wait(&mut self) -> Option<WaitHandle<QueueError<T>>> {
    if self.is_closed() {
      None
    } else {
      Some(self.register_consumer_waiter())
    }
  }

  fn is_closed(&self) -> bool {
    self.is_closed()
  }
}

/// Async queue alias backed by [`EmbassyBoundedMpscBackend`].
pub type EmbassyMpscQueue<T, M, const N: usize> =
  AsyncQueue<T, MpscKey, EmbassyBoundedMpscBackend<T, M, N>, EmbassyAsyncMutex<M, EmbassyBoundedMpscBackend<T, M, N>>>;

/// Constructs an embassy-backed MPSC queue with the specified raw mutex and capacity.
pub fn make_embassy_mpsc_queue_with_mutex<T, M, const N: usize>() -> EmbassyMpscQueue<T, M, N>
where
  M: RawMutex,
  T: Send,
  MpscKey: MultiProducer, {
  let backend = EmbassyBoundedMpscBackend::<T, M, N>::new();
  let shared = ArcShared::new(EmbassyAsyncMutex::new(backend));
  AsyncQueue::<T, MpscKey, _, _>::new_mpsc(shared)
}

/// Constructs an embassy-backed MPSC queue using [`NoopRawMutex`].
pub fn make_embassy_mpsc_queue<T, const N: usize>() -> EmbassyMpscQueue<T, NoopRawMutex, N>
where
  T: Send,
  MpscKey: MultiProducer, {
  make_embassy_mpsc_queue_with_mutex::<T, NoopRawMutex, N>()
}

/// Convenience alias using [`CriticalSectionRawMutex`] for interrupt-safe contexts.
pub type EmbassyCsMpscQueue<T, const N: usize> = EmbassyMpscQueue<T, CriticalSectionRawMutex, N>;
