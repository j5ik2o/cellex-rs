use alloc::boxed::Box;
use core::marker::PhantomData;

use cellex_actor_core_rs::shared::mailbox::MailboxSignal;
use cellex_utils_embedded_rs::sync::arc::ArcShared;
use embassy_sync::{blocking_mutex::raw::RawMutex, signal::Signal};

use super::signal_wait::DefaultSignalWait;

/// Notification primitive used to wake mailbox waiters.
pub struct DefaultSignal<RM>
where
  RM: RawMutex, {
  signal: ArcShared<Signal<RM, ()>>,
}

impl<RM> Clone for DefaultSignal<RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self { signal: self.signal.clone() }
  }
}

impl<RM> Default for DefaultSignal<RM>
where
  RM: RawMutex,
{
  fn default() -> Self {
    Self { signal: ArcShared::new(Signal::new()) }
  }
}

impl<RM> DefaultSignal<RM>
where
  RM: RawMutex,
{
  pub(crate) fn new() -> Self {
    Self::default()
  }
}

impl<RM> MailboxSignal for DefaultSignal<RM>
where
  RM: RawMutex,
{
  type WaitFuture<'a>
    = DefaultSignalWait<'a, RM>
  where
    Self: 'a;

  fn notify(&self) {
    self.signal.signal(());
  }

  fn wait(&self) -> Self::WaitFuture<'_> {
    DefaultSignalWait { future: Box::pin(self.signal.wait()), _marker: PhantomData }
  }
}
