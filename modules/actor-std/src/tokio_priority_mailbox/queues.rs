#[cfg(feature = "queue-v1")]
mod legacy {
  use std::{collections::VecDeque, sync::Arc};

  use cellex_actor_core_rs::{api::metrics::MetricsSinkShared, shared::mailbox::messages::PriorityEnvelope};
  use cellex_utils_std_rs::{
    sync::{StdMutexGuard, StdSyncMutex},
    Element, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter,
  };

  fn lock_mutex<'a, T>(mutex: &'a StdSyncMutex<T>) -> StdMutexGuard<'a, T> {
    mutex.lock()
  }

  fn lock_arc_mutex<'a, T>(mutex: &'a Arc<StdSyncMutex<T>>) -> StdMutexGuard<'a, T> {
    mutex.lock()
  }

  pub(super) struct TokioPriorityLevels<M> {
    levels:             Arc<Vec<StdSyncMutex<VecDeque<PriorityEnvelope<M>>>>>,
    capacity_per_level: usize,
  }

  impl<M> Clone for TokioPriorityLevels<M> {
    fn clone(&self) -> Self {
      Self { levels: Arc::clone(&self.levels), capacity_per_level: self.capacity_per_level }
    }
  }

  impl<M> TokioPriorityLevels<M> {
    pub(super) fn new(levels: usize, capacity_per_level: usize) -> Self {
      let storage = (0..levels).map(|_| StdSyncMutex::new(VecDeque::new())).collect();
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
    regular:          Arc<StdSyncMutex<VecDeque<PriorityEnvelope<M>>>>,
    regular_capacity: usize,
  }

  impl<M> TokioPriorityQueues<M> {
    pub(crate) fn new(levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
      Self {
        control: TokioPriorityLevels::new(levels, control_per_level),
        regular: Arc::new(StdSyncMutex::new(VecDeque::new())),
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

  impl<M> QueueBase<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn len(&self) -> QueueSize {
      self.len()
    }

    fn capacity(&self) -> QueueSize {
      self.capacity()
    }
  }

  impl<M> QueueWriter<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn offer_mut(&mut self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
      self.offer(envelope)
    }
  }

  impl<M> QueueReader<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn poll_mut(&mut self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
      self.poll()
    }

    fn clean_up_mut(&mut self) {
      self.clean_up();
    }
  }

  impl<M> QueueRw<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
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

  pub(crate) fn configure_metrics<M>(_queues: &TokioPriorityQueues<M>, _sink: Option<MetricsSinkShared>)
  where
    M: Element, {
  }
}

#[cfg(feature = "queue-v2")]
mod compat {
  use cellex_actor_core_rs::{
    api::metrics::MetricsSinkShared,
    shared::mailbox::{messages::PriorityEnvelope, queue_rw_compat::QueueRwCompat},
  };
  use cellex_utils_core_rs::v2::collections::queue::backend::OverflowPolicy;
  use cellex_utils_std_rs::{Element, QueueBase, QueueError, QueueReader, QueueRw, QueueSize, QueueWriter};

  pub(super) struct TokioPriorityLevels<M>
  where
    M: Element, {
    queues: Vec<QueueRwCompat<PriorityEnvelope<M>>>,
  }

  impl<M> TokioPriorityLevels<M>
  where
    M: Element,
  {
    pub(super) fn new(levels: usize, capacity_per_level: usize) -> Self {
      let level_count = levels.max(1);
      let queues = (0..level_count)
        .map(|_| match capacity_per_level {
          | 0 => QueueRwCompat::unbounded(),
          | capacity => QueueRwCompat::bounded(capacity, OverflowPolicy::Block),
        })
        .collect();
      Self { queues }
    }

    fn level_index(priority: i8, levels: usize) -> usize {
      let max = (levels.saturating_sub(1)) as i8;
      priority.clamp(0, max) as usize
    }

    #[allow(clippy::result_large_err)]
    pub(super) fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
      let idx = Self::level_index(envelope.priority(), self.queues.len());
      self.queues[idx].offer(envelope)
    }

    #[allow(clippy::result_large_err, clippy::unnecessary_wraps)]
    pub(super) fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
      for level in (0..self.queues.len()).rev() {
        match self.queues[level].poll()? {
          | Some(envelope) => return Ok(Some(envelope)),
          | None => continue,
        }
      }
      Ok(None)
    }

    pub(super) fn clean_up(&self) {
      for queue in &self.queues {
        queue.clean_up();
      }
    }

    pub(super) fn len(&self) -> usize {
      self.queues.iter().map(|queue| queue.len().to_usize()).sum()
    }

    pub(super) fn capacity(&self) -> QueueSize {
      let mut total = 0usize;
      for queue in &self.queues {
        let capacity = queue.capacity();
        if capacity.is_limitless() {
          return QueueSize::limitless();
        }
        total = total.saturating_add(capacity.to_usize());
      }
      QueueSize::limited(total)
    }

    pub(super) fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
      for queue in &self.queues {
        queue.set_metrics_sink(sink.clone());
      }
    }
  }

  impl<M> Clone for TokioPriorityLevels<M>
  where
    M: Element,
  {
    fn clone(&self) -> Self {
      Self { queues: self.queues.clone() }
    }
  }

  pub struct TokioPriorityQueues<M>
  where
    M: Element, {
    control: TokioPriorityLevels<M>,
    regular: QueueRwCompat<PriorityEnvelope<M>>,
  }

  impl<M> TokioPriorityQueues<M>
  where
    M: Element,
  {
    pub(crate) fn new(levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
      let regular = match regular_capacity {
        | 0 => QueueRwCompat::unbounded(),
        | capacity => QueueRwCompat::bounded(capacity, OverflowPolicy::Block),
      };
      Self { control: TokioPriorityLevels::new(levels, control_per_level), regular }
    }

    #[allow(clippy::result_large_err)]
    fn offer(&self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
      if envelope.is_control() {
        self.control.offer(envelope)
      } else {
        self.regular.offer(envelope)
      }
    }

    #[allow(clippy::result_large_err, clippy::unnecessary_wraps)]
    fn poll(&self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
      if let Some(envelope) = self.control.poll()? {
        return Ok(Some(envelope));
      }
      self.regular.poll()
    }

    fn clean_up(&self) {
      self.control.clean_up();
      self.regular.clean_up();
    }

    fn len_queue_size(&self) -> QueueSize {
      let control_len = self.control.len();
      let regular_len = self.regular.len().to_usize();
      QueueSize::limited(control_len.saturating_add(regular_len))
    }

    fn capacity_queue_size(&self) -> QueueSize {
      let control_cap = self.control.capacity();
      let regular_cap = self.regular.capacity();
      if control_cap.is_limitless() || regular_cap.is_limitless() {
        QueueSize::limitless()
      } else {
        let total = control_cap.to_usize().saturating_add(regular_cap.to_usize());
        QueueSize::limited(total)
      }
    }

    pub(super) fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>) {
      self.control.set_metrics_sink(sink.clone());
      self.regular.set_metrics_sink(sink);
    }
  }

  impl<M> QueueBase<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn len(&self) -> QueueSize {
      self.len_queue_size()
    }

    fn capacity(&self) -> QueueSize {
      self.capacity_queue_size()
    }
  }

  impl<M> QueueWriter<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn offer_mut(&mut self, envelope: PriorityEnvelope<M>) -> Result<(), QueueError<PriorityEnvelope<M>>> {
      self.offer(envelope)
    }
  }

  impl<M> QueueReader<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn poll_mut(&mut self) -> Result<Option<PriorityEnvelope<M>>, QueueError<PriorityEnvelope<M>>> {
      self.poll()
    }

    fn clean_up_mut(&mut self) {
      self.clean_up();
    }
  }

  impl<M> Clone for TokioPriorityQueues<M>
  where
    M: Element,
  {
    fn clone(&self) -> Self {
      Self { control: self.control.clone(), regular: self.regular.clone() }
    }
  }

  impl<M> QueueRw<PriorityEnvelope<M>> for TokioPriorityQueues<M>
  where
    M: Element,
  {
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

  pub(crate) fn configure_metrics<M>(queues: &TokioPriorityQueues<M>, sink: Option<MetricsSinkShared>)
  where
    M: Element, {
    queues.set_metrics_sink(sink);
  }
}

#[cfg(feature = "queue-v2")]
pub(super) use compat::{configure_metrics, TokioPriorityQueues};
#[cfg(feature = "queue-v1")]
pub(super) use legacy::{configure_metrics, TokioPriorityQueues};
