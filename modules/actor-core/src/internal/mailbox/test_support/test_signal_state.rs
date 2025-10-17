#[derive(Clone, Default)]
pub(crate) struct TestSignalState {
  pub(crate) notified: bool,
  pub(crate) waker: Option<core::task::Waker>,
}
