use core::{
  future::Future,
  pin::Pin,
  ptr,
  task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use super::{AsyncMpscQueue, AsyncQueue, AsyncSpscQueue};
use crate::{
  sync::{async_mutex_like::SpinAsyncMutex, ArcShared},
  v2::collections::queue::{
    backend::{OfferOutcome, OverflowPolicy, QueueError, VecRingBackend},
    type_keys::{MpscKey, SpscKey},
    VecRingStorage,
  },
};

fn block_on<F: Future>(mut future: F) -> F::Output {
  fn raw_waker() -> RawWaker {
    fn clone(_: *const ()) -> RawWaker {
      raw_waker()
    }
    fn wake(_: *const ()) {}
    fn wake_by_ref(_: *const ()) {}
    fn drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    RawWaker::new(ptr::null(), &VTABLE)
  }

  let waker = unsafe { Waker::from_raw(raw_waker()) };
  let mut future = unsafe { Pin::new_unchecked(&mut future) };
  let mut context = Context::from_waker(&waker);

  loop {
    match future.as_mut().poll(&mut context) {
      | Poll::Ready(output) => return output,
      | Poll::Pending => continue,
    }
  }
}

fn make_shared_queue(capacity: usize, policy: OverflowPolicy) -> ArcShared<SpinAsyncMutex<VecRingBackend<i32>>> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  ArcShared::new(SpinAsyncMutex::new(backend))
}

#[test]
fn offer_and_poll_operates_async_queue() {
  let shared = make_shared_queue(4, OverflowPolicy::Block);
  let queue: AsyncSpscQueue<i32, _, _> = AsyncQueue::new_spsc(shared);

  assert_eq!(block_on(queue.is_empty()), true);
  assert!(matches!(block_on(queue.offer(42)), Ok(OfferOutcome::Enqueued)));
  assert_eq!(block_on(queue.len()), 1);
  assert_eq!(block_on(queue.poll()), Ok(42));
  assert_eq!(block_on(queue.is_empty()), true);
}

#[test]
fn into_mpsc_handles_roundtrip() {
  let shared = make_shared_queue(4, OverflowPolicy::Block);
  let queue: AsyncMpscQueue<i32, _, _> = AsyncQueue::new_mpsc(shared);
  let (producer, consumer) = queue.into_mpsc_handles();

  assert!(matches!(block_on(producer.offer(7)), Ok(OfferOutcome::Enqueued)));
  assert_eq!(block_on(consumer.poll()), Ok(7));
}

#[test]
fn close_prevents_further_operations() {
  let shared = make_shared_queue(2, OverflowPolicy::Block);
  let queue: AsyncSpscQueue<i32, _, _> = AsyncQueue::new_spsc(shared);

  assert!(matches!(block_on(queue.offer(1)), Ok(OfferOutcome::Enqueued)));
  assert!(block_on(queue.close()).is_ok());
  assert_eq!(block_on(queue.poll()), Ok(1));
  assert_eq!(block_on(queue.poll()), Err(QueueError::Closed));
  assert_eq!(block_on(queue.offer(2)), Err(QueueError::Closed));
}
