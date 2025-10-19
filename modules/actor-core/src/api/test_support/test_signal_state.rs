#[derive(Clone, Default)]
/// Shared state manipulated by `TestSignal` to track notifications during polling.
pub struct TestSignalState {
  pub(crate) notified: bool,
  pub(crate) waker:    Option<core::task::Waker>,
}

impl TestSignalState {
  /// Creates a state value with an explicit notification flag and stored waker.
  #[must_use]
  pub const fn new(notified: bool, waker: Option<core::task::Waker>) -> Self {
    Self { notified, waker }
  }

  /// Returns whether the signal has been delivered.
  #[must_use]
  pub const fn notified(&self) -> bool {
    self.notified
  }

  /// Returns the waker currently waiting on the signal.
  #[must_use]
  pub const fn waker(&self) -> Option<&core::task::Waker> {
    self.waker.as_ref()
  }
}
