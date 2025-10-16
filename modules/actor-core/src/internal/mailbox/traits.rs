// Re-export mailbox runtime traits from the public API so internal modules can depend on a single path.
pub use crate::{Mailbox, MailboxProducer, MailboxRuntime, SingleThread, ThreadSafe};
