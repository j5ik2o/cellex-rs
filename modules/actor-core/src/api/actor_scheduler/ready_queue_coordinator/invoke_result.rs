//! InvokeResult - Message invocation outcomes

use alloc::string::String;
use core::time::Duration;

use super::{ResumeCondition, SuspendReason};

/// Result of message invocation
#[derive(Debug, Clone, PartialEq)]
pub enum InvokeResult {
  /// Processing completed
  Completed {
    /// Hint: should this mailbox be re-registered to ready queue?
    ready_hint: bool,
  },
  /// Yielded for fairness (reached throughput limit)
  Yielded,
  /// Actor is suspended
  Suspended {
    /// Reason for suspension
    reason:    SuspendReason,
    /// Condition for resuming the actor
    resume_on: ResumeCondition,
  },
  /// Actor failed during processing
  Failed {
    /// Error message (simplified for prototype)
    error:       String,
    /// Optional retry delay
    retry_after: Option<Duration>,
  },
  /// Actor has stopped
  Stopped,
}
