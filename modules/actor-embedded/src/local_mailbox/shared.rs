use core::task::Waker;

#[cfg(not(feature = "embedded_rc"))]
use cellex_utils_embedded_rs::collections::queue::mpsc::ArcLocalMpscUnboundedQueue;
#[cfg(feature = "embedded_rc")]
use cellex_utils_embedded_rs::collections::queue::mpsc::RcMpscUnboundedQueue;
#[cfg(not(feature = "embedded_rc"))]
use cellex_utils_embedded_rs::sync::ArcLocalStateCell;
#[cfg(feature = "embedded_rc")]
use cellex_utils_embedded_rs::sync::{RcShared, RcStateCell};
use cellex_utils_embedded_rs::{Element, StateCell};

#[cfg(feature = "embedded_rc")]
pub(super) type LocalQueueInner<M> = RcShared<RcMpscUnboundedQueue<M>>;

#[cfg(not(feature = "embedded_rc"))]
pub(super) type LocalQueueInner<M> = ArcLocalMpscUnboundedQueue<M>;

#[cfg(feature = "embedded_rc")]
pub(super) fn new_queue<M>() -> LocalQueueInner<M>
where
  M: Element, {
  RcShared::new(RcMpscUnboundedQueue::new())
}

#[cfg(not(feature = "embedded_rc"))]
pub(super) fn new_queue<M>() -> LocalQueueInner<M>
where
  M: Element, {
  ArcLocalMpscUnboundedQueue::new()
}

#[cfg(feature = "embedded_rc")]
pub(super) fn clone_queue<M>(inner: &LocalQueueInner<M>) -> LocalQueueInner<M>
where
  M: Element, {
  inner.clone()
}

#[cfg(not(feature = "embedded_rc"))]
pub(super) fn clone_queue<M>(inner: &LocalQueueInner<M>) -> LocalQueueInner<M>
where
  M: Element, {
  inner.clone()
}

#[cfg(feature = "embedded_rc")]
pub(super) type SignalCell = RcStateCell<SignalState>;

#[cfg(not(feature = "embedded_rc"))]
pub(super) type SignalCell = ArcLocalStateCell<SignalState>;

#[cfg(feature = "embedded_rc")]
pub(super) fn new_signal_cell() -> SignalCell {
  SignalCell::new(SignalState::default())
}

#[cfg(not(feature = "embedded_rc"))]
pub(super) fn new_signal_cell() -> SignalCell {
  SignalCell::new(SignalState::default())
}

#[cfg(feature = "embedded_rc")]
pub(super) fn with_signal_state_mut<T>(state: &SignalCell, f: impl FnOnce(&mut SignalState) -> T) -> T {
  let mut guard = state.borrow_mut();
  f(&mut guard)
}

#[cfg(not(feature = "embedded_rc"))]
pub(super) fn with_signal_state_mut<T>(state: &SignalCell, f: impl FnOnce(&mut SignalState) -> T) -> T {
  let mut guard = state.borrow_mut();
  f(&mut guard)
}

#[derive(Debug, Default)]
pub(super) struct SignalState {
  pub(super) notified: bool,
  pub(super) waker:    Option<Waker>,
}
