//! std-specific helpers for v2 queue abstractions.

use cellex_utils_core_rs::{
  sync::ArcShared,
  v2::collections::queue::{
    facade::Queue as CoreQueue, FifoKey, MpscKey, OverflowPolicy, SpscKey, VecRingBackend, VecRingStorage,
  },
};

use crate::sync::StdSyncMutex;

#[cfg(test)]
mod tests;

/// Queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type StdFifoQueue<T> = CoreQueue<T, FifoKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;
/// MPSC queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type StdMpscQueue<T> = CoreQueue<T, MpscKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;
/// SPSC queue alias backed by [`StdSyncMutex`] and [`VecRingBackend`].
pub type StdSpscQueue<T> = CoreQueue<T, SpscKey, VecRingBackend<T>, StdSyncMutex<VecRingBackend<T>>>;

/// Constructs a [`StdFifoQueue`] with the given capacity and overflow policy.
pub fn make_std_fifo_queue<T>(capacity: usize, policy: OverflowPolicy) -> StdFifoQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::DropOldest`].
pub fn make_std_fifo_queue_drop_oldest<T>(capacity: usize) -> StdFifoQueue<T> {
  make_std_fifo_queue(capacity, OverflowPolicy::DropOldest)
}

/// Constructs an [`StdMpscQueue`] with the given capacity and overflow policy.
pub fn make_std_mpsc_queue<T>(capacity: usize, policy: OverflowPolicy) -> StdMpscQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::DropOldest`] for MPSC usage.
pub fn make_std_mpsc_queue_drop_oldest<T>(capacity: usize) -> StdMpscQueue<T> {
  make_std_mpsc_queue(capacity, OverflowPolicy::DropOldest)
}

/// Constructs an [`StdSpscQueue`] with the given capacity and overflow policy.
pub fn make_std_spsc_queue<T>(capacity: usize, policy: OverflowPolicy) -> StdSpscQueue<T> {
  let storage = VecRingStorage::with_capacity(capacity);
  let backend = VecRingBackend::new_with_storage(storage, policy);
  let shared = ArcShared::new(StdSyncMutex::new(backend));
  CoreQueue::new(shared)
}

/// Convenience constructor that defaults to [`OverflowPolicy::Block`] for SPSC usage.
pub fn make_std_spsc_queue_blocking<T>(capacity: usize) -> StdSpscQueue<T> {
  make_std_spsc_queue(capacity, OverflowPolicy::Block)
}
