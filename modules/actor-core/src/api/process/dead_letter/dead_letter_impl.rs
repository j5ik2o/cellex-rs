//! Dead letter implementation.

use crate::api::process::{dead_letter::DeadLetterReason, pid::Pid};

/// Message captured by the DeadLetter hub.
#[derive(Debug, Clone)]
pub struct DeadLetter<M> {
  /// PID originally targeted by the message.
  pub pid:     Pid,
  /// Original message envelope.
  pub message: M,
  /// Recorded reason.
  pub reason:  DeadLetterReason,
}

impl<M> DeadLetter<M> {
  /// Creates a new dead letter entry.
  pub const fn new(pid: Pid, message: M, reason: DeadLetterReason) -> Self {
    Self { pid, message, reason }
  }
}
