use super::*;
use futures::task::noop_waker_ref;
use std::{task::Context, time::Duration};
use tokio::time::sleep;

#[tokio::test(flavor = "current_thread")]
async fn tokio_deadline_timer_expires_after_duration() {
  let mut queue = TokioDeadlineTimer::new();
  let key = queue
    .insert("hello", TimerDeadline::from(Duration::from_millis(10)))
    .unwrap();

  sleep(Duration::from_millis(20)).await;

  let expired = futures::future::poll_fn(|cx| queue.poll_expired(cx))
    .await
    .expect("expired");
  assert_eq!(expired.item, "hello");
  assert_eq!(expired.key, key);
}

#[tokio::test(flavor = "current_thread")]
async fn tokio_deadline_timer_reset_extends_deadline() {
  let mut queue = TokioDeadlineTimer::new();
  let key = queue
    .insert("value", TimerDeadline::from(Duration::from_secs(1)))
    .unwrap();
  queue.reset(key, TimerDeadline::from(Duration::from_secs(2))).unwrap();

  sleep(Duration::from_millis(1100)).await;

  let waker = noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  assert!(matches!(queue.poll_expired(&mut cx), Poll::Pending));

  sleep(Duration::from_millis(1100)).await;

  let expired = futures::future::poll_fn(|cx| queue.poll_expired(cx))
    .await
    .expect("expired");
  assert_eq!(expired.key, key);
  assert_eq!(expired.item, "value");
}

#[tokio::test(flavor = "current_thread")]
async fn tokio_deadline_timer_cancel_returns_value() {
  let mut queue = TokioDeadlineTimer::new();
  let key = queue
    .insert("cancel", TimerDeadline::from(Duration::from_millis(5)))
    .unwrap();
  let removed = queue.cancel(key).unwrap();
  assert_eq!(removed, Some("cancel"));
  assert!(queue.cancel(key).unwrap().is_none());
}
