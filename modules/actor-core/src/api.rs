/// Actor core types and behavior management.
pub mod actor;
/// Actor runtime trait and generic implementations.
pub mod actor_runtime;
/// Actor system infrastructure and lifecycle management.
pub mod actor_system;
#[cfg(feature = "alloc")]
pub(crate) mod extensions;
/// Failure event stream for telemetry and monitoring.
pub mod failure_event_stream;
/// Actor identity types including ActorId and ActorPath.
pub mod identity;
/// Mailbox implementations and message queueing.
pub mod mailbox;
/// Message envelope and metadata handling.
pub mod messaging;
/// Supervision strategies and failure handling.
pub mod supervision;
