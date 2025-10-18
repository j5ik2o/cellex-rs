extern crate std;

use std::{
  future::Future,
  pin::Pin,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
};

use super::*;

fn noop_waker() -> Waker {
  struct NoopWake;
  impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}

    fn wake_by_ref(self: &Arc<Self>) {}
  }
  Waker::from(Arc::new(NoopWake))
}

#[test]
fn immediate_timer_sleep_is_ready() {
  let timer = ImmediateTimer;
  let mut fut = timer.sleep(Duration::from_secs(1));
  let waker = noop_waker();
  let mut cx = Context::from_waker(&waker);
  assert_eq!(Pin::new(&mut fut).poll(&mut cx), Poll::Ready(()));
}
