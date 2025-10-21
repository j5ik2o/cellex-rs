//! Shared supervision abstractions reused across layers.

/// Escalation sink abstractions for failure propagation.
pub mod escalation_sink; // allow module_wiring::no_parent_reexport
