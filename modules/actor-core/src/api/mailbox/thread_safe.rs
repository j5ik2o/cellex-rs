use crate::api::mailbox::mailbox_concurrency::MailboxConcurrency;

/// Thread-safe mailbox mode requiring `Send + Sync` types.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadSafe;

impl MailboxConcurrency for ThreadSafe {}
