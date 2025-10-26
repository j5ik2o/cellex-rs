//! Shared mailbox abstractions reused across layers.

/// Mailbox factory abstractions for creating mailboxes.
mod factory;
/// Mailbox handle abstractions for runtime scheduler integration.
mod handle;
/// Message envelope types for mailbox communication.
pub mod messages;
/// Mailbox configuration options.
mod options;
/// Mailbox producer abstractions for sending messages.
mod producer;
/// Mailbox signal abstractions for notification mechanisms.
mod signal;

pub use factory::*;
pub use handle::*;
pub use options::*;
pub use producer::*;
pub use signal::*;
