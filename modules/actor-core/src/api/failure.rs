mod failure_event;
mod failure_info;
mod failure_metadata;
/// Additional metadata associated with failures.
pub mod metadata;

pub use failure_event::FailureEvent;
pub use failure_info::FailureInfo;
pub use failure_metadata::FailureMetadata;

/// Failure event stream for telemetry and monitoring.
pub mod failure_event_stream;
/// Shared failure telemetry infrastructure
pub mod failure_telemetry;
#[cfg(test)]
mod tests;
