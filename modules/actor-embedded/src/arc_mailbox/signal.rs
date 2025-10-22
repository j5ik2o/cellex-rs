use alloc::boxed::Box;
use core::marker::PhantomData;

use cellex_actor_core_rs::api::mailbox::MailboxSignal;
use cellex_utils_embedded_rs::sync::ArcShared;
use embassy_sync::{blocking_mutex::raw::RawMutex, signal::Signal};

use super::signal_wait::ArcSignalWait;

/// Notification primitive used to wake mailbox waiters.
pub struct ArcSignal<RM>
where
  RM: RawMutex, {
  signal: ArcShared<Signal<RM, ()>>,
}

impl<RM> Clone for ArcSignal<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self { signal: self.signal.clone() }
  }
}

impl<RM> Default for ArcSignal<RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self { signal: ArcShared::new(Signal::new()) }
  }
}

impl<RM> ArcSignal<RM>
where
  RM: RawMutex,
{
  pub(crate) fn new() -> Self {
    Self::default()
  }
}

impl<RM> MailboxSignal for ArcSignal<RM>
where
  RM: RawMutex,
{
  type WaitFuture<'a>
    = ArcSignalWait<'a, RM>
  where
    Self: 'a;

  fn notify(&self) {
    self.signal.signal(());
  }

  fn wait(&self) -> Self::WaitFuture<'_> {
    ArcSignalWait { future: Box::pin(self.signal.wait()), _marker: PhantomData }
  }
}
