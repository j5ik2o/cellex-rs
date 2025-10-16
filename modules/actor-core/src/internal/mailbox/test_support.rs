#[cfg(test)]
mod tests;

use core::cell::RefCell;
use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, MpscBuffer, MpscHandle, MpscQueue, QueueSize, RingBufferBackend, Shared};

use super::traits::{MailboxPair, MailboxRuntime, MailboxSignal, ThreadSafe};
use crate::{MailboxOptions, QueueMailbox, QueueMailboxProducer};

#[derive(Clone, Debug, Default)]
pub struct TestMailboxRuntime {
  capacity: Option<usize>,
}

impl TestMailboxRuntime {
  pub const fn new(capacity: Option<usize>) -> Self {
    Self { capacity }
  }

  pub const fn with_capacity_per_queue(capacity: usize) -> Self {
    Self::new(Some(capacity))
  }

  pub fn unbounded() -> Self {
    Self::default()
  }

  const fn resolve_capacity(&self, options: MailboxOptions) -> Option<usize> {
    match options.capacity {
      QueueSize::Limitless => self.capacity,
      QueueSize::Limited(value) => Some(value),
    }
  }
}

pub struct SharedBackendHandle<T>(ArcShared<RingBufferBackend<RefCell<MpscBuffer<T>>>>);

impl<T> SharedBackendHandle<T> {
  fn new(capacity: Option<usize>) -> Self {
    let buffer = RefCell::new(MpscBuffer::new(capacity));
    let backend = RingBufferBackend::new(buffer);
    Self(ArcShared::new(backend))
  }
}

impl<T> Clone for SharedBackendHandle<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> core::ops::Deref for SharedBackendHandle<T> {
  type Target = RingBufferBackend<RefCell<MpscBuffer<T>>>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> fmt::Debug for SharedBackendHandle<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SharedBackendHandle").finish()
  }
}

impl<T> Shared<RingBufferBackend<RefCell<MpscBuffer<T>>>> for SharedBackendHandle<T> {
  fn try_unwrap(self) -> Result<RingBufferBackend<RefCell<MpscBuffer<T>>>, Self>
  where
    RingBufferBackend<RefCell<MpscBuffer<T>>>: Sized, {
    match self.0.try_unwrap() {
      Ok(inner) => Ok(inner),
      Err(shared) => Err(Self(shared)),
    }
  }
}

impl<T> MpscHandle<T> for SharedBackendHandle<T> {
  type Backend = RingBufferBackend<RefCell<MpscBuffer<T>>>;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}

pub type TestQueue<M> = MpscQueue<SharedBackendHandle<M>, M>;

#[derive(Clone)]
pub struct TestSignal {
  state: ArcShared<RefCell<TestSignalState>>,
}

impl Default for TestSignal {
  fn default() -> Self {
    Self {
      state: ArcShared::new(RefCell::new(TestSignalState::default())),
    }
  }
}

#[derive(Clone, Default)]
struct TestSignalState {
  notified: bool,
  waker: Option<core::task::Waker>,
}

impl MailboxSignal for TestSignal {
  type WaitFuture<'a>
    = TestSignalWait<'a>
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
    TestSignalWait {
      signal: self.clone(),
      _marker: PhantomData,
    }
  }
}

pub struct TestSignalWait<'a> {
  signal: TestSignal,
  _marker: PhantomData<&'a ()>,
}

impl<'a> Future for TestSignalWait<'a> {
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

impl MailboxRuntime for TestMailboxRuntime {
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
    = TestQueue<M>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    let queue = MpscQueue::new(SharedBackendHandle::new(capacity));
    let signal = TestSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let sender = mailbox.producer();
    (mailbox, sender)
  }
}
