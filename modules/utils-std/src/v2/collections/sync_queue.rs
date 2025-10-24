//! std-specific helpers for v2 queue abstractions.

use cellex_utils_core_rs::{
  sync::ArcShared,
  v2::collections::queue::{
    FifoKey, MpscKey, OverflowPolicy, SpscKey, SyncQueue as CoreSyncQueue, VecRingBackend, VecRingStorage,
  },
};

use crate::sync::StdSyncMutex;

#[cfg(test)]
mod tests;

/// Queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type StdFifoSyncQueue<T> = CoreSyncQueue<T, FifoKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;
/// MPSC queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type StdMpscSyncQueue<T> = CoreSyncQueue<T, MpscKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;
/// SPSC queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type StdSpscSyncQueue<T> = CoreSyncQueue<T, SpscKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;

/// Constructs a [`StdFifoSyncQueue`] with the given capacity and overflow policy.
pub fn make_std_fifo_queue<T>(capacity: usize, policy: OverflowPolicy) -> StdFifoSyncQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreSyncQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::DropOldest`].
pub fn make_std_fifo_queue_drop_oldest<T>(capacity: usize) -> StdFifoSyncQueue<T> {
  make_std_fifo_queue(capacity, OverflowPolicy::DropOldest)
}

/// Constructs an [`StdMpscSyncQueue`] with the given capacity and overflow policy.
pub fn make_std_mpsc_queue<T>(capacity: usize, policy: OverflowPolicy) -> StdMpscSyncQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreSyncQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::DropOldest`] for MPSC usage.
pub fn make_std_mpsc_queue_drop_oldest<T>(capacity: usize) -> StdMpscSyncQueue<T> {
  make_std_mpsc_queue(capacity, OverflowPolicy::DropOldest)
}

/// Constructs an [`StdSpscSyncQueue`] with the given capacity and overflow policy.
pub fn make_std_spsc_queue<T>(capacity: usize, policy: OverflowPolicy) -> StdSpscSyncQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreSyncQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::Block`] for SPSC usage.
pub fn make_std_spsc_queue_blocking<T>(capacity: usize) -> StdSpscSyncQueue<T> {
  make_std_spsc_queue(capacity, OverflowPolicy::Block)
}
