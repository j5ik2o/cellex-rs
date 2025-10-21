//! Additional metadata attached to failure information.

/// Escalation depth and progress tracking.
pub mod failure_escalation_stage; // allow module_wiring::no_parent_reexport

pub use failure_escalation_stage::FailureEscalationStage;
