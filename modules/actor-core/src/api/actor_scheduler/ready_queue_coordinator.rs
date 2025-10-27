//! ReadyQueueCoordinator - Ready queue coordination and signal management
//!
//! This module provides the prototype implementation of ReadyQueueCoordinator
//! as part of Phase 0 of the ActorScheduler refactoring.
//!
//! # Design Goals
//!
//! - Separate ready queue management from scheduler frontend
//! - Provide clear API for register/unregister/drain operations
//! - Enable runtime-agnostic signal handling via poll_wait_signal
//!
//! # References
//!
//! - Design doc: `docs/design/actor_scheduler_refactor.md` Section 4.4
//! - ADR: `docs/adr/2025-10-22-phase0-naming-policy.md`

// Module declarations
mod actor_state;
mod invoke_result;
mod mailbox_index;
mod mailbox_options;
mod overflow_strategy;
mod ready_queue_coordinator_trait;
mod resume_condition;
mod signal_key;
mod suspend_reason;

#[cfg(test)]
mod tests;

// Re-exports
pub use actor_state::ActorState;
pub use invoke_result::InvokeResult;
pub use mailbox_index::MailboxIndex;
pub use overflow_strategy::OverflowStrategy;
pub use ready_queue_coordinator_trait::ReadyQueueCoordinator;
pub use resume_condition::ResumeCondition;
pub use signal_key::SignalKey;
pub use suspend_reason::SuspendReason;
