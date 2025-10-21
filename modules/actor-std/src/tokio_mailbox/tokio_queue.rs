use std::sync::Arc;

use cellex_utils_std_rs::{
  collections::queue::mpsc::{ArcMpscBoundedQueue, ArcMpscUnboundedQueue},
  Element, QueueBase, QueueError, QueueRw, QueueSize,
};

/// Queue implementation for Tokio mailbox
///
/// Supports both bounded and unbounded queues backed by MPSC channels.
#[derive(Debug)]
pub struct TokioQueue<M>
where
  M: Element, {
  inner: Arc<TokioQueueKind<M>>,
}

/// Internal queue kind discriminant
///
/// Represents either an unbounded or bounded MPSC queue.
#[derive(Debug)]
enum TokioQueueKind<M>
where
  M: Element, {
  Unbounded(ArcMpscUnboundedQueue<M>),
  Bounded(ArcMpscBoundedQueue<M>),
}

impl<M> Clone for TokioQueue<M>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { inner: Arc::clone(&self.inner) }
  }
}

impl<M> TokioQueue<M>
where
  M: Element,
{
  pub(super) fn with_capacity(size: QueueSize) -> Self {
    let kind = match size {
      | QueueSize::Limitless => TokioQueueKind::Unbounded(ArcMpscUnboundedQueue::new()),
      | QueueSize::Limited(0) => TokioQueueKind::Unbounded(ArcMpscUnboundedQueue::new()),
      | QueueSize::Limited(capacity) => TokioQueueKind::Bounded(ArcMpscBoundedQueue::new(capacity)),
    };
    Self { inner: Arc::new(kind) }
  }

  fn kind(&self) -> &TokioQueueKind<M> {
    self.inner.as_ref()
  }
}

impl<M> QueueBase<M> for TokioQueue<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    match self.kind() {
      | TokioQueueKind::Unbounded(queue) => queue.len(),
      | TokioQueueKind::Bounded(queue) => queue.len(),
    }
  }

  fn capacity(&self) -> QueueSize {
    match self.kind() {
      | TokioQueueKind::Unbounded(queue) => queue.capacity(),
      | TokioQueueKind::Bounded(queue) => queue.capacity(),
    }
  }
}

impl<M> QueueRw<M> for TokioQueue<M>
where
  M: Element,
{
  fn offer(&self, element: M) -> Result<(), QueueError<M>> {
    match self.kind() {
      | TokioQueueKind::Unbounded(queue) => queue.offer(element),
      | TokioQueueKind::Bounded(queue) => queue.offer(element),
    }
  }

  fn poll(&self) -> Result<Option<M>, QueueError<M>> {
    match self.kind() {
      | TokioQueueKind::Unbounded(queue) => queue.poll(),
      | TokioQueueKind::Bounded(queue) => queue.poll(),
    }
  }

  fn clean_up(&self) {
    match self.kind() {
      | TokioQueueKind::Unbounded(queue) => queue.clean_up(),
      | TokioQueueKind::Bounded(queue) => queue.clean_up(),
    }
  }
}
