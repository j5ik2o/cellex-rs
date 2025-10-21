mod escalation_stage;
mod failure_event;
mod failure_info;
mod failure_metadata;

pub use escalation_stage::EscalationStage;
pub use failure_event::FailureEvent;
pub use failure_info::FailureInfo;
pub use failure_metadata::FailureMetadata;

#[cfg(test)]
mod tests;
/// Failure event stream for telemetry and monitoring.
pub mod failure_event_stream;
/// Shared failure telemetry infrastructure
pub mod failure_telemetry;
