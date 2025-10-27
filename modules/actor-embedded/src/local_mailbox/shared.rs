use core::task::Waker;

use cellex_utils_core_rs::sync::StateCell;
#[cfg(not(feature = "embedded_rc"))]
use cellex_utils_embedded_rs::sync::arc::ArcLocalStateCell;
#[cfg(feature = "embedded_rc")]
use cellex_utils_embedded_rs::sync::rc::RcStateCell;

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
