//! Shared mailbox abstractions reused across layers.

/// Mailbox consumer abstractions for runtime scheduler integration.
mod consumer;
/// Mailbox factory abstractions for creating mailboxes.
mod factory;
/// Message envelope types for mailbox communication.
pub mod messages;
/// Mailbox configuration options.
mod options;
/// Mailbox producer abstractions for sending messages.
mod producer;
/// Mailbox signal abstractions for notification mechanisms.
mod signal;

pub use consumer::*;
pub use factory::*;
pub use options::*;
pub use producer::*;
pub use signal::*;
