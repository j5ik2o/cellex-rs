/// Actor core types and behavior management.
pub mod actor;
/// Actor runtime trait and generic implementations.
pub mod actor_runtime;
/// Actor system infrastructure and lifecycle management.
pub mod actor_system;
#[cfg(feature = "alloc")]
/// Extensions for actor system and actor runtime.
pub mod extensions;
/// Failure event stream for telemetry and monitoring.
pub mod failure_event_stream;
/// Mailbox implementations and message queueing.
pub mod mailbox;
/// Message envelope and metadata handling.
pub mod messaging;
/// Internal metrics collection and observers wired to scheduler components.
pub mod metrics;
/// Supervision strategies and failure handling.
pub mod supervision;
