pub mod builder;
pub mod queue_mailbox;
pub mod spawner;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
#[cfg(test)]
mod tests;
pub mod traits;

#[allow(unused_imports)]
pub use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
#[cfg(any(test, feature = "test-support"))]
#[allow(unused_imports)]
pub use crate::api::mailbox::PriorityChannel;
#[allow(unused_imports)]
pub use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
#[allow(unused_imports)]
pub use crate::MailboxOptions;
pub use builder::PriorityMailboxBuilder;
pub use spawner::PriorityMailboxSpawnerHandle;
