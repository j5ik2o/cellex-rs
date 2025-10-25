#[cfg(test)]
mod tests;

mod factory;
mod mailbox;
#[cfg(any(feature = "queue-v2", not(feature = "queue-v1")))]
mod priority_sync_driver;
#[cfg(not(feature = "queue-v1"))]
mod priority_sync_handle;
#[cfg(feature = "queue-v1")]
mod queues;
mod sender;

pub use factory::ArcPriorityMailboxFactory;
pub use mailbox::ArcPriorityMailbox;
pub use sender::ArcPriorityMailboxSender;
