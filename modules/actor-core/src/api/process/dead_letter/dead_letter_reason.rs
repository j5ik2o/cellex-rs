//! Dead letter reason type.

use core::fmt;

/// Reason why a message was routed to dead letters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadLetterReason {
  /// No process is registered for the given PID.
  UnregisteredPid,
  /// Process exists but is currently terminating or terminated.
  Terminated,
  /// The delivery subsystem rejected the message (e.g., queue full).
  DeliveryRejected,
  /// Remote transport reported a network-level failure for the destination node.
  NetworkUnreachable,
  /// Custom reason text supplied by the caller.
  Custom(&'static str),
}

impl fmt::Display for DeadLetterReason {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | Self::UnregisteredPid => f.write_str("unregistered pid"),
      | Self::Terminated => f.write_str("terminated"),
      | Self::DeliveryRejected => f.write_str("delivery rejected"),
      | Self::NetworkUnreachable => f.write_str("network unreachable"),
      | Self::Custom(msg) => f.write_str(msg),
    }
  }
}
