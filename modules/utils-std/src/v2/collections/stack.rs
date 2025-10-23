//! std-specific helpers for v2 stack abstractions.

use cellex_utils_core_rs::{
  sync::ArcShared,
  v2::collections::stack::{facade::Stack as CoreStack, StackOverflowPolicy, VecStackBackend, VecStackStorage},
};

use crate::sync::StdSyncMutex;

#[cfg(test)]
mod tests;

/// Stack type alias backed by [`StdSyncMutex`] and [`VecStackBackend`].
pub type StdVecStack<T> = CoreStack<T, VecStackBackend<T>, StdSyncMutex<VecStackBackend<T>>>;

/// Constructs a new [`StdVecStack`] with the specified capacity and overflow policy.
pub fn make_std_vec_stack<T>(capacity: usize, policy: StackOverflowPolicy) -> StdVecStack<T> {
  let storage = VecStackStorage::with_capacity(capacity);
  let backend = VecStackBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  StdVecStack::new(shared)
}

/// Convenience constructor that defaults to [`StackOverflowPolicy::Block`].
pub fn make_std_vec_stack_blocking<T>(capacity: usize) -> StdVecStack<T> {
  make_std_vec_stack(capacity, StackOverflowPolicy::Block)
}
