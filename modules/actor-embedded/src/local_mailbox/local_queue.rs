#[cfg(not(feature = "embedded_rc"))]
use cellex_utils_embedded_rs::collections::queue::mpsc::ArcLocalMpscUnboundedQueue;
#[cfg(feature = "embedded_rc")]
use cellex_utils_embedded_rs::collections::queue::mpsc::RcMpscUnboundedQueue;
use cellex_utils_embedded_rs::{Element, QueueBase, QueueError, QueueRw, QueueSize};

use super::shared::{clone_queue, new_queue, LocalQueueInner};

#[derive(Debug)]
pub struct LocalQueue<M>
where
  M: Element, {
  inner: LocalQueueInner<M>,
}

impl<M> LocalQueue<M>
where
  M: Element,
{
  pub(super) fn new() -> Self {
    Self { inner: new_queue() }
  }

  #[cfg(feature = "embedded_rc")]
  fn as_ref(&self) -> &RcMpscUnboundedQueue<M> {
    &self.inner
  }

  #[cfg(not(feature = "embedded_rc"))]
  fn as_ref(&self) -> &ArcLocalMpscUnboundedQueue<M> {
    &self.inner
  }
}

impl<M> Clone for LocalQueue<M>
where
  M: Element,
{
  fn clone(&self) -> Self {
    Self { inner: clone_queue(&self.inner) }
  }
}

impl<M> QueueBase<M> for LocalQueue<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    self.as_ref().len()
  }

  fn capacity(&self) -> QueueSize {
    self.as_ref().capacity()
  }
}

impl<M> QueueRw<M> for LocalQueue<M>
where
  M: Element,
{
  fn offer(&self, element: M) -> Result<(), QueueError<M>> {
    self.as_ref().offer(element)
  }

  fn poll(&self) -> Result<Option<M>, QueueError<M>> {
    self.as_ref().poll()
  }

  fn clean_up(&self) {
    self.as_ref().clean_up();
  }
}
