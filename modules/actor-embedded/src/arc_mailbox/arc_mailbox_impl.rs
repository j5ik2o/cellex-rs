use cellex_actor_core_rs::api::{
  mailbox::{
    queue_mailbox::{QueueMailbox, QueueMailboxRecv},
    Mailbox,
  },
  metrics::MetricsSinkShared,
};
use cellex_utils_embedded_rs::{collections::queue::mpsc::ArcMpscUnboundedQueue, Element, QueueError, QueueSize};
use embassy_sync::blocking_mutex::raw::RawMutex;

use super::{runtime::ArcMailboxRuntime, sender::ArcMailboxSender, signal::ArcSignal};

/// Mailbox implementation backed by an `ArcShared` MPSC queue.
#[derive(Clone)]
pub struct ArcMailbox<M, RM = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>
where
  M: Element,
  RM: RawMutex, {
  pub(crate) inner: QueueMailbox<ArcMpscUnboundedQueue<M, RM>, ArcSignal<RM>>,
}

impl<M, RM> ArcMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
{
  /// Creates an unbounded mailbox and sender pair.
  pub fn new() -> (Self, ArcMailboxSender<M, RM>) {
    ArcMailboxRuntime::<RM>::new().unbounded()
  }

  /// Returns the underlying queue mailbox.
  pub fn inner(&self) -> &QueueMailbox<ArcMpscUnboundedQueue<M, RM>, ArcSignal<RM>> {
    &self.inner
  }

  /// Updates the metrics sink associated with the mailbox.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}

impl<M, RM> Mailbox<M> for ArcMailbox<M, RM>
where
  M: Element,
  RM: RawMutex,
  ArcMpscUnboundedQueue<M, RM>: Clone,
{
  type RecvFuture<'a>
    = QueueMailboxRecv<'a, ArcMpscUnboundedQueue<M, RM>, ArcSignal<RM>, M>
  where
    Self: 'a;
  type SendError = QueueError<M>;

  fn try_send(&self, message: M) -> Result<(), Self::SendError> {
    self.inner.try_send(message)
  }

  fn recv(&self) -> Self::RecvFuture<'_> {
    self.inner.recv()
  }

  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }

  fn close(&self) {
    self.inner.close();
  }

  fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.inner.set_metrics_sink(sink);
  }
}
