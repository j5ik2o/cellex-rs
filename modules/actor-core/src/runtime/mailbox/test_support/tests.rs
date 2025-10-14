use super::*;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use super::super::traits::Mailbox;

#[test]
fn test_mailbox_runtime_delivers_fifo() {
  let factory = TestMailboxRuntime::with_capacity_per_queue(2);
  let (mailbox, sender) = factory.build_default_mailbox::<u32>();

  sender.try_send(1).unwrap();
  sender.try_send(2).unwrap();

  let mut future = mailbox.recv();
  let waker = noop_waker();
  let mut cx = Context::from_waker(&waker);
  let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

  assert_eq!(pinned.as_mut().poll(&mut cx), Poll::Ready(Ok(1)));
  assert_eq!(pinned.poll(&mut cx), Poll::Ready(Ok(2)));
}

fn noop_waker() -> Waker {
  unsafe { Waker::from_raw(noop_raw_waker()) }
}

fn noop_raw_waker() -> RawWaker {
  fn clone(_: *const ()) -> RawWaker {
    noop_raw_waker()
  }
  fn wake(_: *const ()) {}
  fn wake_by_ref(_: *const ()) {}
  fn drop(_: *const ()) {}

  RawWaker::new(core::ptr::null(), &RawWakerVTable::new(clone, wake, wake_by_ref, drop))
}
