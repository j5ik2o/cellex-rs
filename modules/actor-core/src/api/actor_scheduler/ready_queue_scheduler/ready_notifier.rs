use cellex_utils_core_rs::sync::ArcShared;
use spin::Mutex;

use super::{ready_event_hook::ReadyEventHook, ready_queue_state::ReadyQueueState};

pub(crate) struct ReadyNotifier {
  state: ArcShared<Mutex<ReadyQueueState>>,
  index: usize,
}

impl ReadyNotifier {
  pub(crate) const fn new(state: ArcShared<Mutex<ReadyQueueState>>, index: usize) -> Self {
    Self { state, index }
  }
}

impl ReadyEventHook for ReadyNotifier {
  fn notify_ready(&self) {
    let mut state = self.state.lock();
    let _ = state.enqueue_if_idle(self.index);
  }
}
