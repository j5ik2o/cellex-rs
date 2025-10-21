//! Shared mailbox message primitives.

pub mod priority_envelope; // allow module_wiring::no_parent_reexport

pub use priority_envelope::PriorityEnvelope;
