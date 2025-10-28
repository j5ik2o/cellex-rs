#![allow(clippy::disallowed_types)]
#![allow(clippy::unwrap_used)]

use core::{
  future::Future,
  pin::Pin,
  task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use super::*;
use crate::{api::mailbox::Mailbox, shared::mailbox::MailboxFactory};

#[test]
fn test_mailbox_factory_delivers_fifo() {
  let mailbox_factory = TestMailboxFactory::with_capacity_per_queue(2);
  let (mailbox, sender) = mailbox_factory.build_default_mailbox::<u32>();

  assert_eq!(mailbox.len_usize::<u32>(), 0);
  assert_eq!(mailbox.capacity_usize::<u32>(), 2);

  sender.try_send(1).unwrap();
  sender.try_send(2).unwrap();

  assert_eq!(mailbox.len_usize::<u32>(), 2);

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
