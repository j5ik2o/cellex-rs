use core::marker::PhantomData;

use crate::{
  sync::{sync_mutex_like::SyncMutexLike, ArcShared},
  v2::{
    collections::queue::backend::{QueueBackend, QueueError},
    sync::SharedAccess,
  },
};

/// Consumer for queues tagged with
/// [`SpscKey`](crate::v2::collections::queue::type_keys::SpscKey).
pub struct SpscConsumer<T, B, M>
where
  B: QueueBackend<T>,
  M: SyncMutexLike<B>, {
  pub(crate) inner: ArcShared<M>,
  _pd:              PhantomData<(T, B)>,
}

impl<T, B, M> SpscConsumer<T, B, M>
where
  B: QueueBackend<T>,
  M: SyncMutexLike<B>,
  ArcShared<M>: SharedAccess<B>,
{
  pub(crate) fn new(inner: ArcShared<M>) -> Self {
    Self { inner, _pd: PhantomData }
  }

  /// Polls the next element from the queue.
  pub fn poll(&self) -> Result<T, QueueError> {
    let result = self.inner.with_mut(|backend: &mut B| backend.poll()).map_err(QueueError::from)?;
    result
  }

  /// Signals that no more elements will be produced.
  pub fn close(&self) {
    let _ = self.inner.with_mut(|backend: &mut B| {
      backend.close();
    });
  }
}
