use super::SuspensionClock;

/// Clock implementation that never reports timestamps.
#[derive(Default)]
pub struct NullSuspensionClock;

impl SuspensionClock for NullSuspensionClock {
  fn now(&self) -> Option<u64> {
    None
  }
}
