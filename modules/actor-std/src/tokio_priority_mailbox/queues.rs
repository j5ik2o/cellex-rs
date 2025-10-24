use cellex_actor_core_rs::shared::mailbox::{messages::PriorityEnvelope, queue_rw_compat::QueueRwCompat};
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
  pub(super) fn new(levels: usize, control_per_level: usize, regular_capacity: usize) -> Self {
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
