use core::cmp;

use crate::v2::collections::queue::{
  OfferOutcome, OverflowPolicy, QueueError, QueueStorage, SyncQueueBackend, VecRingStorage,
};

/// Queue backend backed by a ring buffer storage.
pub struct VecRingBackend<T> {
  storage: VecRingStorage<T>,
  policy:  OverflowPolicy,
  closed:  bool,
}

impl<T> VecRingBackend<T> {
  /// Creates a backend from the provided storage and overflow policy.
  #[must_use]
  pub const fn new_with_storage(storage: VecRingStorage<T>, policy: OverflowPolicy) -> Self {
    Self { storage, policy, closed: false }
  }

  fn ensure_capacity(&mut self, required: usize) -> Result<Option<usize>, QueueError> {
    if required <= self.storage.capacity() {
      return Ok(None);
    }

    let current = self.storage.capacity();
    let next = cmp::max(required, cmp::max(1, current.saturating_mul(2)));
    self.storage.try_grow(next).map_err(|_| QueueError::AllocError)?;
    Ok(Some(next))
  }

  fn handle_full_queue(&mut self, item: T) -> Result<OfferOutcome, QueueError> {
    match self.policy {
      | OverflowPolicy::DropNewest => {
        drop(item);
        Ok(OfferOutcome::DroppedNewest { count: 1 })
      },
      | OverflowPolicy::DropOldest => {
        let _ = self.storage.pop_front();
        self.storage.push_back(item);
        Ok(OfferOutcome::DroppedOldest { count: 1 })
      },
      | OverflowPolicy::Block => Err(QueueError::Full),
      | OverflowPolicy::Grow => {
        let grown_to = self.handle_grow_policy(item)?;
        Ok(OfferOutcome::GrewTo { capacity: grown_to })
      },
    }
  }

  fn handle_grow_policy(&mut self, item: T) -> Result<usize, QueueError> {
    let required = self.storage.len().saturating_add(1);
    if let Some(capacity) = self.ensure_capacity(required)? {
      self.storage.push_back(item);
      Ok(capacity)
    } else {
      self.storage.push_back(item);
      Ok(self.storage.capacity())
    }
  }
}

impl<T> SyncQueueBackend<T> for VecRingBackend<T> {
  type Storage = VecRingStorage<T>;

  fn new(storage: Self::Storage, policy: OverflowPolicy) -> Self {
    VecRingBackend::new_with_storage(storage, policy)
  }

  fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError> {
    if self.closed {
      return Err(QueueError::Closed);
    }

    if self.storage.is_full() {
      return self.handle_full_queue(item);
    }

    self.storage.push_back(item);
    Ok(OfferOutcome::Enqueued)
  }

  fn poll(&mut self) -> Result<T, QueueError> {
    match self.storage.pop_front() {
      | Some(item) => Ok(item),
      | None => {
        if self.closed {
          Err(QueueError::Closed)
        } else {
          Err(QueueError::Empty)
        }
      },
    }
  }

  fn len(&self) -> usize {
    self.storage.len()
  }

  fn capacity(&self) -> usize {
    self.storage.capacity()
  }

  fn close(&mut self) {
    self.closed = true;
  }
}
