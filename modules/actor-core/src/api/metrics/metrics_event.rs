/// Metrics events emitted by the actor runtime.
///
/// The variants currently cover high-level categories; payloads can be extended in later phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsEvent {
  /// An actor was registered with the scheduler.
  ActorRegistered,
  /// An actor was deregistered from the scheduler.
  ActorDeregistered,
  /// A user message was enqueued into a mailbox.
  MailboxEnqueued,
  /// A message was dequeued from a mailbox.
  MailboxDequeued,
  /// A mailbox evicted the oldest messages to make room for new arrivals.
  MailboxDroppedOldest {
    /// Number of messages that were evicted.
    count: usize,
  },
  /// A mailbox rejected the newest messages due to overflow policy.
  MailboxDroppedNewest {
    /// Number of messages that were rejected.
    count: usize,
  },
  /// A mailbox grew its storage capacity to accommodate more messages.
  MailboxGrewTo {
    /// New capacity after growth.
    capacity: usize,
  },
  /// An actor mailbox entered a suspended state.
  MailboxSuspended {
    /// Total number of observed suspend events for the mailbox.
    suspend_count: u64,
  },
  /// An actor mailbox resumed message processing.
  MailboxResumed {
    /// Total number of observed resume events for the mailbox.
    resume_count:   u64,
    /// Duration of the most recent suspension, if available.
    last_duration:  Option<core::time::Duration>,
    /// Cumulative suspension duration, if available.
    total_duration: Option<core::time::Duration>,
  },
  /// Telemetry handling logic was invoked.
  TelemetryInvoked,
  /// Duration, in nanoseconds, spent executing telemetry handlers.
  TelemetryLatencyNanos(u64),
}
