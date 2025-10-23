use alloc::boxed::Box;
use core::marker::PhantomData;

use async_trait::async_trait;

use super::{AsyncStackBackend, PushOutcome, StackBackend, StackError};

/// Adapter that exposes a synchronous stack backend through the async backend trait.
pub struct SyncAdapterStackBackend<T, B>
where
  B: StackBackend<T>, {
  backend: B,
  _pd:     PhantomData<T>,
}

impl<T, B> SyncAdapterStackBackend<T, B>
where
  B: StackBackend<T>,
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
impl<T, B> AsyncStackBackend<T> for SyncAdapterStackBackend<T, B>
where
  B: StackBackend<T>,
{
  async fn push(&mut self, item: T) -> Result<PushOutcome, StackError> {
    self.backend.push(item)
  }

  async fn pop(&mut self) -> Result<T, StackError> {
    self.backend.pop()
  }

  fn peek(&self) -> Option<&T> {
    self.backend.peek()
  }

  async fn close(&mut self) -> Result<(), StackError> {
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
