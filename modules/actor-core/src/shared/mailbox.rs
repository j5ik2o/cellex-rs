//! Shared mailbox abstractions reused across layers.

/// Mailbox factory abstractions for creating mailboxes.
pub mod factory; // allow module_wiring::no_parent_reexport
/// Mailbox handle abstractions for runtime scheduler integration.
pub mod handle; // allow module_wiring::no_parent_reexport
/// Message envelope types for mailbox communication.
pub mod messages; // allow module_wiring::no_parent_reexport
/// Mailbox configuration options.
pub mod options; // allow module_wiring::no_parent_reexport
/// Mailbox producer abstractions for sending messages.
pub mod producer; // allow module_wiring::no_parent_reexport
/// Mailbox signal abstractions for notification mechanisms.
pub mod signal; // allow module_wiring::no_parent_reexport

#[cfg(feature = "queue-v2")]
/// Compatibility adapter bridging v2 queues with legacy traits.
pub mod queue_rw_compat; // allow module_wiring::no_parent_reexport
