use alloc::{collections::VecDeque, vec::Vec};

pub(crate) struct ReadyQueueState {
  pub(crate) queue:   VecDeque<usize>,
  pub(crate) queued:  Vec<bool>,
  pub(crate) running: Vec<bool>,
}

impl ReadyQueueState {
  pub(super) fn new() -> Self {
    Self { queue: VecDeque::new(), queued: Vec::new(), running: Vec::new() }
  }

  pub(super) fn ensure_capacity(&mut self, len: usize) {
    if self.queued.len() < len {
      self.queued.resize(len, false);
    }
    if self.running.len() < len {
      self.running.resize(len, false);
    }
  }

  pub(super) fn enqueue_if_idle(&mut self, index: usize) -> bool {
    self.ensure_capacity(index + 1);
    if self.running[index] || self.queued[index] {
      return false;
    }
    self.queue.push_back(index);
    self.queued[index] = true;
    true
  }

  pub(super) fn mark_running(&mut self, index: usize) {
    self.ensure_capacity(index + 1);
    self.running[index] = true;
    if index < self.queued.len() {
      self.queued[index] = false;
    }
  }

  pub(super) fn mark_idle(&mut self, index: usize, has_pending: bool) {
    self.ensure_capacity(index + 1);
    self.running[index] = false;
    if has_pending {
      let _ = self.enqueue_if_idle(index);
    }
  }
}
