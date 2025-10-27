use crate::api::mailbox::queue_mailbox::SyncMailboxQueue;

/// Queue abstraction backed by v2 collections when the `queue-v2` feature is enabled.
pub type TestQueue<M> = SyncMailboxQueue<M>;
