//! std-specific helpers for v2 queue abstractions.

use cellex_utils_core_rs::{
  collections::queue::{
    backend::{OverflowPolicy, VecRingBackend},
    storage::VecRingStorage,
    type_keys::{FifoKey, MpscKey, SpscKey},
    SyncQueue as CoreSyncQueue,
  },
  sync::ArcShared,
};

use crate::sync::StdSyncMutex;

#[cfg(test)]
mod tests;

/// Queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type SyncStdFifoQueue<T> = CoreSyncQueue<T, FifoKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;
/// MPSC queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type SyncStdMpscQueue<T> = CoreSyncQueue<T, MpscKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;
/// SPSC queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type SyncStdSpscQueue<T> = CoreSyncQueue<T, SpscKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;

/// Constructs a [`SyncStdFifoQueue`] with the given capacity and overflow policy.
pub fn make_std_fifo_queue<T>(capacity: usize, policy: OverflowPolicy) -> SyncStdFifoQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreSyncQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::DropOldest`].
pub fn make_std_fifo_queue_drop_oldest<T>(capacity: usize) -> SyncStdFifoQueue<T> {
  make_std_fifo_queue(capacity, OverflowPolicy::DropOldest)
}

/// Constructs an [`SyncStdMpscQueue`] with the given capacity and overflow policy.
pub fn make_std_mpsc_queue<T>(capacity: usize, policy: OverflowPolicy) -> SyncStdMpscQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreSyncQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::DropOldest`] for MPSC usage.
pub fn make_std_mpsc_queue_drop_oldest<T>(capacity: usize) -> SyncStdMpscQueue<T> {
  make_std_mpsc_queue(capacity, OverflowPolicy::DropOldest)
}

/// Constructs an [`SyncStdSpscQueue`] with the given capacity and overflow policy.
pub fn make_std_spsc_queue<T>(capacity: usize, policy: OverflowPolicy) -> SyncStdSpscQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreSyncQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::Block`] for SPSC usage.
pub fn make_std_spsc_queue_blocking<T>(capacity: usize) -> SyncStdSpscQueue<T> {
  make_std_spsc_queue(capacity, OverflowPolicy::Block)
}
