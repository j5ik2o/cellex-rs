extern crate std;

use core::task::Poll;
use std::{task::Context, time::Duration};

use futures::task::noop_waker_ref;

use super::*;

#[test]
fn manual_deadline_timer_expires_after_advance() {
  let mut queue = ManualDeadlineTimer::new();
  let key = queue.insert("timeout", TimerDeadline::from(Duration::from_millis(10))).unwrap();

  let waker = noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  assert!(matches!(queue.poll_expired(&mut cx), Poll::Pending));

  queue.advance(Duration::from_millis(5));
  assert!(matches!(queue.poll_expired(&mut cx), Poll::Pending));

  queue.advance(Duration::from_millis(5));
  let expired = queue.poll_expired(&mut cx).map(|res| res.unwrap());
  assert!(matches!(expired, Poll::Ready(exp) if exp.key == key && exp.item == "timeout"));
}

#[test]
fn manual_deadline_timer_cancel_and_reset() {
  let mut queue = ManualDeadlineTimer::new();
  let key = queue.insert("value", TimerDeadline::from(Duration::from_millis(5))).unwrap();

  let cancelled = queue.cancel(key).unwrap();
  assert_eq!(cancelled, Some("value"));
  assert!(queue.cancel(key).unwrap().is_none());

  let key = queue.insert("reset", TimerDeadline::from(Duration::from_millis(5))).unwrap();
  queue.advance(Duration::from_millis(5));

  queue.reset(key, TimerDeadline::from(Duration::from_millis(10))).unwrap();

  let waker = noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  assert!(matches!(queue.poll_expired(&mut cx), Poll::Pending));
  queue.advance(Duration::from_millis(10));
  let expired = queue.poll_expired(&mut cx).map(|res| res.unwrap());
  assert!(matches!(expired, Poll::Ready(exp) if exp.key == key && exp.item == "reset"));
}
