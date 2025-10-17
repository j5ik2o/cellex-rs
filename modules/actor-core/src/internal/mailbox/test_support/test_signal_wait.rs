use crate::internal::mailbox::test_support::test_signal::TestSignal;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct TestSignalWait<'a> {
  pub(crate) signal: TestSignal,
  pub(crate) _marker: PhantomData<&'a ()>,
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
