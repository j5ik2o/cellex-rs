#![cfg(feature = "embassy_executor")]

use core::time::Duration;

use cellex_actor_core_rs::api::receive_timeout::ReceiveTimeoutScheduler;

use super::internal::{StateMutex, WakeSignal};

/// Receive-timeout scheduler used by the Embassy runtime.
///
/// Commands (`set` / `cancel` / `notify_activity`) update shared state immediately and
/// wake the background task, which drives the actual Embassy timer.
pub struct EmbassyReceiveTimeoutScheduler {
  state:       &'static StateMutex,
  wake_signal: &'static WakeSignal,
}

impl EmbassyReceiveTimeoutScheduler {
  pub(super) fn new(state: &'static StateMutex, wake_signal: &'static WakeSignal) -> Self {
    Self { state, wake_signal }
  }
}

impl ReceiveTimeoutScheduler for EmbassyReceiveTimeoutScheduler {
  fn set(&mut self, duration: Duration) {
    self.state.lock(|state| {
      state.set_duration(Some(duration));
      state.increment_generation();
    });
    self.wake_signal.signal(());
  }

  fn cancel(&mut self) {
    self.state.lock(|state| {
      state.set_duration(None);
      state.increment_generation();
    });
    self.wake_signal.signal(());
  }

  fn notify_activity(&mut self) {
    let should_signal = self.state.lock(|state| {
      if state.get_duration().is_some() {
        state.increment_generation();
        true
      } else {
        false
      }
    });
    if should_signal {
      self.wake_signal.signal(());
    }
  }
}
