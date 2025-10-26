use alloc::vec::Vec;

use cellex_utils_core_rs::{
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
  v2::collections::queue::{
    backend::{OfferOutcome, OverflowPolicy, QueueError, VecRingBackend},
    storage::VecRingStorage,
    SharedVecRingQueue,
  },
};

pub(crate) struct ReadyQueueState {
  queue:              SharedVecRingQueue<usize>,
  pub(crate) queued:  Vec<bool>,
  pub(crate) running: Vec<bool>,
}

impl ReadyQueueState {
  pub(crate) fn new() -> Self {
    let storage = VecRingStorage::with_capacity(0);
    let backend = VecRingBackend::new_with_storage(storage, OverflowPolicy::Grow);
    let shared_backend = ArcShared::new(SpinSyncMutex::new(backend));
    let queue = SharedVecRingQueue::new(shared_backend);
    Self { queue, queued: Vec::new(), running: Vec::new() }
  }

  pub(crate) fn ensure_capacity(&mut self, len: usize) {
    if self.queued.len() < len {
      self.queued.resize(len, false);
    }
    if self.running.len() < len {
      self.running.resize(len, false);
    }
  }

  pub(crate) fn enqueue_if_idle(&mut self, index: usize) -> bool {
    self.ensure_capacity(index + 1);
    if self.running[index] || self.queued[index] {
      return false;
    }
    match self.queue.offer(index) {
      | Ok(OfferOutcome::Enqueued) | Ok(OfferOutcome::GrewTo { .. }) => {
        self.queued[index] = true;
        true
      },
      | Ok(OfferOutcome::DroppedOldest { .. }) | Ok(OfferOutcome::DroppedNewest { .. }) => {
        debug_assert!(false, "ready queue should not drop entries under grow policy");
        false
      },
      | Err(QueueError::Full(_))
      | Err(QueueError::OfferError(_))
      | Err(QueueError::WouldBlock)
      | Err(QueueError::AllocError(_)) => false,
      | Err(QueueError::Closed(_)) | Err(QueueError::Disconnected) | Err(QueueError::Empty) => {
        debug_assert!(false, "ready queue backend returned unexpected state");
        false
      },
    }
  }

  pub(crate) fn mark_running(&mut self, index: usize) {
    self.ensure_capacity(index + 1);
    self.running[index] = true;
    if index < self.queued.len() {
      self.queued[index] = false;
    }
  }

  pub(crate) fn mark_idle(&mut self, index: usize, has_pending: bool) {
    self.ensure_capacity(index + 1);
    self.running[index] = false;
    if has_pending {
      let _ = self.enqueue_if_idle(index);
    }
  }

  pub(crate) fn pop_front(&mut self) -> Option<usize> {
    match self.queue.poll() {
      | Ok(index) => {
        if index < self.queued.len() {
          self.queued[index] = false;
        }
        Some(index)
      },
      | Err(QueueError::Empty) => None,
      | Err(QueueError::Closed(_)) | Err(QueueError::Disconnected) => None,
      | Err(QueueError::Full(_))
      | Err(QueueError::OfferError(_))
      | Err(QueueError::WouldBlock)
      | Err(QueueError::AllocError(_)) => {
        debug_assert!(false, "ready queue backend returned unexpected error on poll");
        None
      },
    }
  }
}
