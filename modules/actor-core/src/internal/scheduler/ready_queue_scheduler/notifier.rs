#![allow(missing_docs)]

use spin::Mutex;

use cellex_utils_core_rs::sync::ArcShared;

use super::hook::ReadyEventHook;
use super::state::ReadyQueueState;

pub(super) struct ReadyNotifier {
  state: ArcShared<Mutex<ReadyQueueState>>,
  index: usize,
}

impl ReadyNotifier {
  pub(super) fn new(state: ArcShared<Mutex<ReadyQueueState>>, index: usize) -> Self {
    Self { state, index }
  }
}

impl ReadyEventHook for ReadyNotifier {
  fn notify_ready(&self) {
    let mut state = self.state.lock();
    let _ = state.enqueue_if_idle(self.index);
  }
}
