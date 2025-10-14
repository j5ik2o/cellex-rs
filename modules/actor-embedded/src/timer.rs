use core::future::Ready;
use core::time::Duration;

use cellex_actor_core_rs::Timer;

/// A timer that completes immediately.
///
/// A timer implementation for embedded environments that completes instantly without waiting.
pub struct ImmediateTimer;

impl Timer for ImmediateTimer {
  type SleepFuture<'a>
    = Ready<()>
  where
    Self: 'a;

  fn sleep(&self, _duration: Duration) -> Self::SleepFuture<'_> {
    core::future::ready(())
  }
}

#[cfg(test)]
mod tests;
