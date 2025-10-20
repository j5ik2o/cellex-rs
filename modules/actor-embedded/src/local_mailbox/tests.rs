#![allow(clippy::disallowed_types)]
extern crate alloc;
extern crate std;

use alloc::{format, string::String};
use core::task::{Context, Poll};
use std::{future::Future, pin::Pin, sync::Arc, task::Wake};

use super::*;

type TestResult<T = ()> = Result<T, String>;

fn noop_waker() -> core::task::Waker {
  struct NoopWake;
  impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}

    fn wake_by_ref(self: &Arc<Self>) {}
  }
  core::task::Waker::from(Arc::new(NoopWake))
}

fn pin_poll<F>(mut fut: F) -> (Poll<F::Output>, F)
where
  F: Future + Unpin, {
  let waker = noop_waker();
  let mut cx = Context::from_waker(&waker);
  let poll = Pin::new(&mut fut).poll(&mut cx);
  (poll, fut)
}

#[test]
fn local_mailbox_delivers_messages_in_fifo_order() -> TestResult {
  let (mailbox, sender) = LocalMailbox::<u32>::new();
  sender.try_send(1).map_err(|err| format!("enqueue first message: {:?}", err))?;
  sender.try_send(2).map_err(|err| format!("enqueue second message: {:?}", err))?;

  let future = mailbox.recv();
  let (first_poll, future) = pin_poll(future);
  assert_eq!(first_poll, Poll::Ready(Ok(1)));

  let (second_poll, _) = pin_poll(future);
  assert_eq!(second_poll, Poll::Ready(Ok(2)));
  Ok(())
}

#[test]
fn local_mailbox_wakes_after_message_arrives() -> TestResult {
  let (mailbox, sender) = LocalMailbox::<u8>::new();

  let mut future = mailbox.recv();
  let waker = noop_waker();
  let mut cx = Context::from_waker(&waker);
  let mut pinned = unsafe { Pin::new_unchecked(&mut future) };

  assert!(pinned.as_mut().poll(&mut cx).is_pending());

  sender.try_send(99_u8).map_err(|err| format!("enqueue message: {:?}", err))?;

  assert_eq!(pinned.poll(&mut cx), Poll::Ready(Ok(99)));
  Ok(())
}

#[test]
fn local_mailbox_preserves_messages_post_wake() -> TestResult {
  let (mailbox, sender) = LocalMailbox::<u8>::new();

  let mut recv_future = mailbox.recv();
  let waker = noop_waker();
  let mut cx = Context::from_waker(&waker);
  let mut pinned = unsafe { Pin::new_unchecked(&mut recv_future) };

  assert!(pinned.as_mut().poll(&mut cx).is_pending());
  sender.try_send(7_u8).map_err(|err| format!("enqueue message: {:?}", err))?;

  let value = pinned.poll(&mut cx);
  assert_eq!(value, Poll::Ready(Ok(7)));
  Ok(())
}

#[test]
fn runtime_builder_produces_working_mailbox() -> TestResult {
  let mailbox_factory = LocalMailboxRuntime::new();
  let (mailbox, sender) = mailbox_factory.unbounded::<u16>();

  sender.try_send(11).map_err(|err| format!("enqueue message: {:?}", err))?;
  let future = mailbox.recv();
  let (poll, _) = pin_poll(future);
  assert_eq!(poll, Poll::Ready(Ok(11)));
  assert!(mailbox.capacity().is_limitless());
  assert_eq!(mailbox.len().to_usize(), 0);
  Ok(())
}
