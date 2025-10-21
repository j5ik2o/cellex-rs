use core::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use super::{local_signal::LocalSignal, shared::with_signal_state_mut};

pub struct LocalSignalWait {
  pub(super) signal: LocalSignal,
}

impl Future for LocalSignalWait {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    with_signal_state_mut(&self.signal.state, |state| {
      if state.notified {
        state.notified = false;
        Poll::Ready(())
      } else {
        state.waker = Some(cx.waker().clone());
        Poll::Pending
      }
    })
  }
}
