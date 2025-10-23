use core::{
  future::Future,
  pin::Pin,
  ptr,
  task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use super::AsyncStack;
use crate::{
  sync::{async_mutex_like::SpinAsyncMutex, ArcShared},
  v2::collections::stack::{
    backend::{PushOutcome, StackError, SyncAdapterStackBackend, VecStackBackend},
    StackOverflowPolicy, VecStackStorage,
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

fn make_shared_stack(
  capacity: usize,
  policy: StackOverflowPolicy,
) -> ArcShared<SpinAsyncMutex<SyncAdapterStackBackend<i32, VecStackBackend<i32>>>> {
  let storage = VecStackStorage::with_capacity(capacity);
  let backend = VecStackBackend::new_with_storage(storage, policy);
  ArcShared::new(SpinAsyncMutex::new(SyncAdapterStackBackend::new(backend)))
}

#[test]
fn push_and_pop_operates_async_stack() {
  let shared = make_shared_stack(4, StackOverflowPolicy::Block);
  let stack: AsyncStack<i32, _, _> = AsyncStack::new(shared);

  assert!(matches!(block_on(stack.push(10)), Ok(PushOutcome::Pushed)));
  assert_eq!(block_on(stack.len()), 1);
  assert_eq!(block_on(stack.pop()), Ok(10));
  assert_eq!(block_on(stack.pop()), Err(StackError::Empty));
}

#[test]
fn peek_reflects_top_element() {
  let shared = make_shared_stack(4, StackOverflowPolicy::Block);
  let stack: AsyncStack<i32, _, _> = AsyncStack::new(shared);

  assert!(matches!(block_on(stack.push(1)), Ok(PushOutcome::Pushed)));
  assert!(matches!(block_on(stack.push(2)), Ok(PushOutcome::Pushed)));
  assert_eq!(block_on(stack.peek()), Ok(Some(2)));
  assert_eq!(block_on(stack.len()), 2);
}

#[test]
fn close_prevents_additional_pushes() {
  let shared = make_shared_stack(2, StackOverflowPolicy::Block);
  let stack: AsyncStack<i32, _, _> = AsyncStack::new(shared);

  assert!(matches!(block_on(stack.push(5)), Ok(PushOutcome::Pushed)));
  assert!(block_on(stack.close()).is_ok());
  assert_eq!(block_on(stack.push(6)), Err(StackError::Closed));
  assert_eq!(block_on(stack.pop()), Ok(5));
  assert_eq!(block_on(stack.pop()), Err(StackError::Closed));
}
