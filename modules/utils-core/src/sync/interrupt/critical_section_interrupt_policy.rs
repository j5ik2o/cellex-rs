use super::InterruptContextPolicy;
use crate::v2::sync::SharedError;

/// Policy placeholder that currently treats all contexts as blocking-capable.
pub struct CriticalSectionInterruptPolicy;

impl InterruptContextPolicy for CriticalSectionInterruptPolicy {
  fn check_blocking_allowed() -> Result<(), SharedError> {
    Ok(())
  }
}
