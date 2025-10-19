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
  /// Telemetry handling logic was invoked.
  TelemetryInvoked,
  /// Duration, in nanoseconds, spent executing telemetry handlers.
  TelemetryLatencyNanos(u64),
}
