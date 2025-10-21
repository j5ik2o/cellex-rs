use cellex_actor_core_rs::api::mailbox::MailboxSignal;

use super::{
  local_signal_wait::LocalSignalWait,
  shared::{new_signal_cell, with_signal_state_mut, SignalCell},
};

#[derive(Clone, Debug)]
pub struct LocalSignal {
  pub(super) state: SignalCell,
}

impl Default for LocalSignal {
  fn default() -> Self {
    Self { state: new_signal_cell() }
  }
}

impl MailboxSignal for LocalSignal {
  type WaitFuture<'a>
    = LocalSignalWait
  where
    Self: 'a;

  fn notify(&self) {
    with_signal_state_mut(&self.state, |state| {
      state.notified = true;
      if let Some(waker) = state.waker.take() {
        waker.wake();
      }
    });
  }

  fn wait(&self) -> Self::WaitFuture<'_> {
    LocalSignalWait { signal: self.clone() }
  }
}
