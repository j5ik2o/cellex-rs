#[derive(Clone, Default)]
/// Shared state manipulated by `TestSignal` to track notifications during polling.
pub struct TestSignalState {
  pub(crate) notified: bool,
  pub(crate) waker:    Option<core::task::Waker>,
}

impl TestSignalState {
  /// Creates a state value with an explicit notification flag and stored waker.
  pub fn new(notified: bool, waker: Option<core::task::Waker>) -> Self {
    Self { notified, waker }
  }

  /// Returns whether the signal has been delivered.
  pub fn notified(&self) -> bool {
    self.notified
  }

  /// Returns the waker currently waiting on the signal.
  pub fn waker(&self) -> Option<&core::task::Waker> {
    self.waker.as_ref()
  }
}
