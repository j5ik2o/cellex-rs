pub mod builder;
mod messages;
mod queue_mailbox;
pub mod spawner;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
#[cfg(test)]
mod tests;
pub mod traits;

#[allow(unused_imports)]
pub use crate::runtime::traits::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
pub use builder::PriorityMailboxBuilder;
#[cfg(any(test, feature = "test-support"))]
pub use messages::PriorityChannel;
pub use messages::{PriorityEnvelope, SystemMessage};
pub use queue_mailbox::{MailboxOptions, QueueMailbox, QueueMailboxProducer, QueueMailboxRecv};
pub use spawner::PriorityMailboxSpawnerHandle;
#[allow(unused_imports)]
pub use traits::{Mailbox, MailboxPair, MailboxRuntime, MailboxSignal};
