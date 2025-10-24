use alloc::boxed::Box;
use core::marker::PhantomData;

use async_trait::async_trait;

use super::{AsyncPriorityBackend, AsyncQueueBackend, OfferOutcome, PriorityBackend, QueueError, SyncQueueBackend};

/// Adapter that exposes a synchronous queue backend through the async backend trait.
pub struct SyncAdapterQueueBackend<T, B>
where
  B: SyncQueueBackend<T>, {
  backend: B,
  _pd:     PhantomData<T>,
}

impl<T, B> SyncAdapterQueueBackend<T, B>
where
  B: SyncQueueBackend<T>,
{
  /// Creates a new adapter wrapping the provided backend instance.
  #[must_use]
  pub const fn new(backend: B) -> Self {
    Self { backend, _pd: PhantomData }
  }

  /// Consumes the adapter and returns the inner backend.
  #[must_use]
  pub fn into_inner(self) -> B {
    self.backend
  }

  /// Provides immutable access to the wrapped backend.
  #[must_use]
  pub fn backend(&self) -> &B {
    &self.backend
  }

  /// Provides mutable access to the wrapped backend.
  #[must_use]
  pub fn backend_mut(&mut self) -> &mut B {
    &mut self.backend
  }
}

#[async_trait(?Send)]
impl<T, B> AsyncQueueBackend<T> for SyncAdapterQueueBackend<T, B>
where
  B: SyncQueueBackend<T>,
{
  async fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError> {
    self.backend.offer(item)
  }

  async fn poll(&mut self) -> Result<T, QueueError> {
    self.backend.poll()
  }

  async fn close(&mut self) -> Result<(), QueueError> {
    self.backend.close();
    Ok(())
  }

  fn len(&self) -> usize {
    self.backend.len()
  }

  fn capacity(&self) -> usize {
    self.backend.capacity()
  }
}

impl<T: Ord, B> AsyncPriorityBackend<T> for SyncAdapterQueueBackend<T, B>
where
  B: PriorityBackend<T>,
{
  fn peek_min(&self) -> Option<&T> {
    self.backend.peek_min()
  }
}
