use crate::internal::scheduler::receive_timeout_scheduler::ReceiveTimeoutScheduler;

/// `ReceiveTimeoutScheduler` implementation that performs no scheduling.
#[derive(Default)]
pub(crate) struct NoopReceiveTimeoutScheduler;

impl ReceiveTimeoutScheduler for NoopReceiveTimeoutScheduler {
  fn set(&mut self, _duration: core::time::Duration) {}

  fn cancel(&mut self) {}

  fn notify_activity(&mut self) {}
}
