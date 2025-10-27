use core::time::Duration;

use cellex_actor_core_rs::api::actor::Timer;
use tokio::time::Sleep;

/// Tokio-backed timer implementation.
pub struct TokioTimer;

impl Timer for TokioTimer {
  type SleepFuture<'a>
    = Sleep
  where
    Self: 'a;

  fn sleep(&self, duration: Duration) -> Self::SleepFuture<'_> {
    tokio::time::sleep(duration)
  }
}
