use crate::api::mailbox::mailbox_concurrency::MailboxConcurrency;

/// Single-threaded mailbox mode without additional synchronization requirements.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleThread;

impl MailboxConcurrency for SingleThread {}
