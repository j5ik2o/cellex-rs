use alloc::boxed::Box;
use core::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use embassy_sync::blocking_mutex::raw::RawMutex;

/// Future returned by [`super::signal::ArcSignal::wait`] that resolves when a signal arrives.
pub struct ArcSignalWait<'a, RM>
where
  RM: RawMutex, {
  pub(super) future:  Pin<Box<dyn Future<Output = ()> + 'a>>,
  pub(super) _marker: PhantomData<RM>,
}

impl<'a, RM> Future for ArcSignalWait<'a, RM>
where
  RM: RawMutex,
{
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = unsafe { self.get_unchecked_mut() };
    this.future.as_mut().poll(cx)
  }
}
