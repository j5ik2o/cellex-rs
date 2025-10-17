use crate::ReceiveTimeoutScheduler;

/// `ReceiveTimeoutScheduler` implementation that performs no scheduling.
#[derive(Default)]
pub struct NoopReceiveTimeoutScheduler;

impl ReceiveTimeoutScheduler for NoopReceiveTimeoutScheduler {
  fn set(&mut self, _duration: core::time::Duration) {}

  fn cancel(&mut self) {}

  fn notify_activity(&mut self) {}
}
