pub mod builder;
mod messages;
mod queue_mailbox;
pub mod spawner;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
#[cfg(test)]
mod tests;
pub mod traits;

pub use builder::PriorityMailboxBuilder;
#[cfg(any(test, feature = "test-support"))]
pub use messages::PriorityChannel;
pub use messages::{PriorityEnvelope, SystemMessage};
pub use queue_mailbox::{MailboxOptions, QueueMailbox, QueueMailboxProducer, QueueMailboxRecv};
pub use spawner::PriorityMailboxSpawnerHandle;
pub use traits::{Mailbox, MailboxPair, MailboxRuntime, MailboxSignal};
