use std::{
  collections::VecDeque,
  sync::{Arc, Mutex, MutexGuard},
};

use cellex_actor_core_rs::shared::mailbox::messages::PriorityEnvelope;
use cellex_utils_std_rs::{QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter};

fn lock_mutex<'a, T>(mutex: &'a Mutex<T>) -> MutexGuard<'a, T> {
  mutex.lock().unwrap_or_else(|err| err.into_inner())
}

fn lock_arc_mutex<'a, T>(mutex: &'a Arc<Mutex<T>>) -> MutexGuard<'a, T> {
  mutex.lock().unwrap_or_else(|err| err.into_inner())
}

pub(super) struct TokioPriorityLevels<M> {
  levels:             Arc<Vec<Mutex<VecDeque<PriorityEnvelope<M>>>>>,
  capacity_per_level: usize,
}

impl<M> Clone for TokioPriorityLevels<M> {
  fn clone(&self) -> Self {
    Self { levels: Arc::clone(&self.levels), capacity_per_level: self.capacity_per_level }
  }
}

impl<M> TokioPriorityLevels<M> {
  pub(super) fn new(levels: usize, capacity_per_level: usize) -> Self {
    let storage = (0..levels).map(|_| Mutex::new(VecDeque::new())).collect();
    Self { levels: Arc::new(storage), capacity_per_level }
  }

  fn level_index(priority: i8, levels: usize) -> usize {
    let max = (levels.saturating_sub(1)) as i8;
    priority.clamp(0, max) as usize
  }

  #[allow(clippy::result_large_err)]
  pub(super) fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    let idx = Self::level_index(envelope.priority(), self.levels.len());
    let mut guard = lock_mutex(&self.levels[idx]);
    if self.capacity_per_level > 0 && guard.len() >= self.capacity_per_level {
      Err(QueueError::Full(envelope))
    } else {
      guard.push_back(envelope);
      Ok(())
    }
  }

  #[allow(clippy::result_large_err, clippy::unnecessary_wraps)]
  pub(super) fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    for level in (0..self.levels.len()).rev() {
      let mut guard = lock_mutex(&self.levels[level]);
      if let Some(envelope) = guard.pop_front() {
        return Ok(Some(envelope));
      }
    }
    Ok(None)
  }

  pub(super) fn clean_up(&self) {
    for level in self.levels.iter() {
      let mut guard = lock_mutex(level);
      guard.clear();
    }
  }

  pub(super) fn len(&self) -> usize {
    self.levels.iter().map(|level| lock_mutex(level).len()).sum()
  }

  pub(super) fn capacity(&self) -> QueueSize {
    if self.capacity_per_level == 0 {
      QueueSize::limitless()
    } else {
      let levels = self.levels.len().max(1);
      QueueSize::limited(self.capacity_per_level * levels)
    }
  }
}

pub struct TokioPriorityQueues<M> {
  control:          TokioPriorityLevels<M>,
  regular:          Arc<Mutex<VecDeque<PriorityEnvelope<M>>>>,
  regular_capacity: usize,
}

impl<M> TokioPriorityQueues<M> {
  pub(super) fn new(levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
    Self {
      control: TokioPriorityLevels::new(levels, control_per_level),
      regular: Arc::new(Mutex::new(VecDeque::new())),
      regular_capacity,
    }
  }

  #[allow(clippy::result_large_err)]
  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    if envelope.is_control() {
      self.control.offer(envelope)
    } else {
      let mut guard = lock_arc_mutex(&self.regular);
      if self.regular_capacity > 0 && guard.len() >= self.regular_capacity {
        Err(QueueError::Full(envelope))
      } else {
        guard.push_back(envelope);
        Ok(())
      }
    }
  }

  #[allow(clippy::result_large_err, clippy::unnecessary_wraps)]
  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    if let Some(envelope) = self.control.poll()? {
      return Ok(Some(envelope));
    }
    let mut guard = lock_arc_mutex(&self.regular);
    Ok(guard.pop_front())
  }

  fn clean_up(&self) {
    self.control.clean_up();
    let mut guard = lock_arc_mutex(&self.regular);
    guard.clear();
  }

  fn len(&self) -> QueueSize {
    let control_len = self.control.len();
    let regular_len = lock_arc_mutex(&self.regular).len();
    QueueSize::limited(control_len.saturating_add(regular_len))
  }

  fn capacity(&self) -> QueueSize {
    let control_cap = self.control.capacity();
    let regular_cap =
      if self.regular_capacity == 0 { QueueSize::limitless() } else { QueueSize::limited(self.regular_capacity) };

    if control_cap.is_limitless() || regular_cap.is_limitless() {
      QueueSize::limitless()
    } else {
      let total = control_cap.to_usize().saturating_add(regular_cap.to_usize());
      QueueSize::limited(total)
    }
  }
}

impl<M> Clone for TokioPriorityQueues<M> {
  fn clone(&self) -> Self {
    Self {
      control:          self.control.clone(),
      regular:          Arc::clone(&self.regular),
      regular_capacity: self.regular_capacity,
    }
  }
}

impl<M> QueueBase<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn len(&self) -> QueueSize {
    self.len()
  }

  fn capacity(&self) -> QueueSize {
    self.capacity()
  }
}

impl<M> QueueWriter<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn offer_mut(&mut self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.offer(envelope)
  }
}

impl<M> QueueReader<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn poll_mut(&mut self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.poll()
  }

  fn clean_up_mut(&mut self) {
    self.clean_up();
  }
}

impl<M> QueueRw<PriorityEnvelope<M>> for TokioPriorityQueues<M> {
  fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.offer(envelope)
  }

  fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
    self.poll()
  }

  fn clean_up(&self) {
    self.clean_up();
  }
}
