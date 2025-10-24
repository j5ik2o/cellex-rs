#![allow(deprecated)]

use std::sync::Mutex;

use cellex_utils_core_rs::{
  QueueSize, Stack, StackBase, StackBuffer, StackError, StackMut, StackStorage, StackStorageBackend,
};

use crate::sync::ArcShared;

#[cfg(test)]
mod tests;

type ArcStackStorage<T> = ArcShared<StackStorageBackend<ArcShared<Mutex<StackBuffer<T>>>>>;

/// `Arc`-based thread-safe stack implementation.
///
/// # Deprecated
/// Prefer [`utils_std::v2::collections::StdVecSyncStack`].
///
/// Provides a stack data structure that can be safely shared across multiple threads.
/// Internally uses `Arc` and `Mutex` for synchronization.
#[derive(Debug, Clone)]
pub struct ArcStack<T> {
  inner: Stack<ArcStackStorage<T>, T>,
}

impl<T> ArcStack<T> {
  /// Creates a new `ArcStack`
  ///
  /// # Returns
  ///
  /// An empty `ArcStack` instance
  #[must_use]
  pub fn new() -> Self {
    let storage = ArcShared::new(Mutex::new(StackBuffer::new()));
    let backend: ArcStackStorage<T> = ArcShared::new(StackStorageBackend::new(storage));
    Self { inner: Stack::new(backend) }
  }

  /// Creates an `ArcStack` with the specified capacity
  ///
  /// # Arguments
  ///
  /// * `capacity` - Maximum capacity of the stack
  ///
  /// # Returns
  ///
  /// An empty `ArcStack` instance with the specified capacity
  #[must_use]
  pub fn with_capacity(capacity: usize) -> Self {
    let stack = Self::new();
    stack.set_capacity(Some(capacity));
    stack
  }

  /// Sets the capacity of the stack
  ///
  /// # Arguments
  ///
  /// * `capacity` - Maximum capacity of the stack. `None` for unlimited capacity
  pub fn set_capacity(&self, capacity: Option<usize>) {
    self.inner.set_capacity(capacity);
  }

  /// Pushes a value onto the stack
  ///
  /// # Arguments
  ///
  /// * `value` - The value to push
  ///
  /// # Returns
  ///
  /// `Ok(())` on success, `Err(StackError)` if capacity limit is exceeded
  ///
  /// # Errors
  ///
  /// Returns `StackError` if the stack has reached its capacity limit
  pub fn push(&self, value: T) -> Result<(), StackError<T>> {
    self.inner.push(value)
  }

  /// Pops a value from the stack
  ///
  /// # Returns
  ///
  /// `Some(T)` if the stack is not empty, `None` if empty
  #[must_use]
  pub fn pop(&self) -> Option<T> {
    self.inner.pop()
  }

  /// Gets the top element without removing it
  ///
  /// # Returns
  ///
  /// A clone of the top element `Some(T)` if the stack is not empty, `None` if empty
  #[must_use]
  pub fn peek(&self) -> Option<T>
  where
    T: Clone, {
    self.inner.peek()
  }

  /// Removes all elements from the stack
  pub fn clear(&self) {
    self.inner.clear();
  }

  /// Returns the number of elements in the stack
  ///
  /// # Returns
  ///
  /// The current number of elements in the stack
  #[must_use]
  pub fn len(&self) -> QueueSize {
    self.inner.len()
  }

  /// Returns the capacity of the stack
  ///
  /// # Returns
  ///
  /// The maximum capacity of the stack. `QueueSize::Unlimited` if unlimited
  #[must_use]
  pub fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<T> Default for ArcStack<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T> StackBase<T> for ArcStack<T> {
  fn len(&self) -> QueueSize {
    self.inner.len()
  }

  fn capacity(&self) -> QueueSize {
    self.inner.capacity()
  }
}

impl<T> StackMut<T> for ArcStack<T> {
  fn push(&mut self, value: T) -> Result<(), StackError<T>> {
    self.inner.push(value)
  }

  fn pop(&mut self) -> Option<T> {
    self.inner.pop()
  }

  fn clear(&mut self) {
    self.inner.clear();
  }

  fn peek(&self) -> Option<T>
  where
    T: Clone, {
    self.inner.peek()
  }
}

impl<T> StackStorage<T> for ArcShared<Mutex<StackBuffer<T>>> {
  fn with_read<R>(&self, f: impl FnOnce(&StackBuffer<T>) -> R) -> R {
    #![allow(clippy::expect_used)]
    let guard = self.lock().expect("mutex poisoned");
    f(&guard)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut StackBuffer<T>) -> R) -> R {
    #![allow(clippy::expect_used)]
    let mut guard = self.lock().expect("mutex poisoned");
    f(&mut guard)
  }
}
