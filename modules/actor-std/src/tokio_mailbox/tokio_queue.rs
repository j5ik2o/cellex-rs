#[cfg(feature = "queue-v2")]
use cellex_utils_core_rs::v2::collections::queue::backend::OverflowPolicy;
use cellex_utils_std_rs::{Element, QueueSize};

#[cfg(feature = "queue-v1")]
mod legacy {
  use std::sync::Arc;

  use cellex_utils_std_rs::{
    collections::queue::mpsc::{ArcMpscBoundedQueue, ArcMpscUnboundedQueue},
    Element, QueueBase, QueueError, QueueRw, QueueSize,
  };

  #[derive(Debug)]
  pub struct TokioQueue<M>
  where
    M: Element, {
    inner: Arc<TokioQueueKind<M>>,
  }

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
    pub(crate) fn with_capacity(size: QueueSize) -> Self {
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
}

#[cfg(feature = "queue-v1")]
pub(super) use legacy::TokioQueue;

#[cfg(feature = "queue-v1")]
pub(super) fn create_tokio_queue<M>(size: QueueSize) -> TokioQueue<M>
where
  M: Element, {
  TokioQueue::with_capacity(size)
}

#[cfg(feature = "queue-v2")]
pub(super) type TokioQueue<M> = cellex_actor_core_rs::shared::mailbox::queue_rw_compat::QueueRwCompat<M>;

#[cfg(feature = "queue-v2")]
pub(super) fn create_tokio_queue<M>(size: QueueSize) -> TokioQueue<M>
where
  M: Element, {
  match size {
    | QueueSize::Limitless | QueueSize::Limited(0) => TokioQueue::unbounded(),
    | QueueSize::Limited(capacity) => TokioQueue::bounded(capacity, OverflowPolicy::Block),
  }
}
