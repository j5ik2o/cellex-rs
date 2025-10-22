//! ResumeCondition - Conditions for resuming suspended actors

use super::SignalKey;
use core::time::Duration;

/// Condition for resuming a suspended actor
#[derive(Debug, Clone, PartialEq)]
pub enum ResumeCondition {
  /// Resume when external signal is received
  ExternalSignal(SignalKey),
  /// Resume after specified duration
  After(Duration),
  /// Resume when capacity becomes available
  WhenCapacityAvailable,
}
