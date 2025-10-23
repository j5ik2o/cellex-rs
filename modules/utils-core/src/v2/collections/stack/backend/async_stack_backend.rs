use alloc::boxed::Box;

use async_trait::async_trait;

use super::{PushOutcome, StackError};

/// Async-compatible backend trait for stack operations.
#[async_trait(?Send)]
pub trait AsyncStackBackend<T> {
  /// Pushes an element onto the stack.
  async fn push(&mut self, item: T) -> Result<PushOutcome, StackError>;

  /// Pops the top element from the stack.
  async fn pop(&mut self) -> Result<T, StackError>;

  /// Returns a reference to the top element without removing it.
  fn peek(&self) -> Option<&T>;

  /// Transitions the backend into the closed state.
  async fn close(&mut self) -> Result<(), StackError>;

  /// Returns the number of stored elements.
  fn len(&self) -> usize;

  /// Returns the storage capacity.
  fn capacity(&self) -> usize;
}
