//! Shared supervision abstractions used across API and internal layers.

/// Escalation sink for failure handling.
mod escalation_sink;

pub use escalation_sink::{EscalationSink, FailureEventHandler};
