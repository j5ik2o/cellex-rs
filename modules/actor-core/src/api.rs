/// Actor core types and behavior management.
pub mod actor;
/// Actor runtime trait and generic implementations.
pub mod actor_runtime;
/// Internal schedulers coordinating actor execution and supervision.
pub mod actor_scheduler;
/// Actor system infrastructure and lifecycle management.
pub mod actor_system;
#[cfg(feature = "alloc")]
/// Extensions for actor system and actor runtime.
pub mod extensions;
/// Failure event stream for telemetry and monitoring.
pub mod failure_event_stream;
/// Shared failure telemetry infrastructure
pub mod failure_telemetry;
/// Mailbox implementations and message queueing.
pub mod mailbox;
/// Message envelope and metadata handling.
pub mod messaging;
/// Internal metrics collection and observers wired to scheduler components.
pub mod metrics;
/// Receive timeout handling
pub mod receive_timeout;
/// Supervision strategies and failure handling.
pub mod supervision;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
