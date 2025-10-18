use core::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use crate::api::test_support::test_signal::TestSignal;

/// Future returned by `TestSignal` that resolves once a notification arrives.
pub struct TestSignalWait<'a> {
  pub(crate) signal:  TestSignal,
  pub(crate) _marker: PhantomData<&'a ()>,
}

impl<'a> TestSignalWait<'a> {
  /// Creates a wait future from the given signal.
  pub fn new(signal: TestSignal) -> Self {
    Self { signal, _marker: PhantomData }
  }
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
