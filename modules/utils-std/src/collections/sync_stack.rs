//! std-specific helpers for v2 stack abstractions.

use cellex_utils_core_rs::{
  collections::stack::{
    backend::{StackOverflowPolicy, VecStackBackend},
    storage::VecStackStorage,
    SyncStack as CoreSyncStack,
  },
  sync::ArcShared,
};

use crate::sync::StdSyncMutex;

#[cfg(test)]
mod tests;

/// Stack type alias backed by [`StdSyncMutex`] and [`VecStackBackend`].
pub type SyncStdVecStack<T> = CoreSyncStack<T, VecStackBackend<T>, StdSyncMutex<VecStackBackend<T>>>;

/// Constructs a new [`SyncStdVecStack`] with the specified capacity and overflow policy.
pub fn make_std_vec_stack<T>(capacity: usize, policy: StackOverflowPolicy) -> SyncStdVecStack<T> {
  let storage = VecStackStorage::with_capacity(capacity);
  let backend = VecStackBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  SyncStdVecStack::new(shared)
}

/// Convenience constructor that defaults to [`StackOverflowPolicy::Block`].
pub fn make_std_vec_stack_blocking<T>(capacity: usize) -> SyncStdVecStack<T> {
  make_std_vec_stack(capacity, StackOverflowPolicy::Block)
}
