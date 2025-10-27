#[cfg(test)]
mod tests;

mod factory;
mod mailbox;
mod priority_mailbox_queue;
mod sender;

pub use factory::DefaultPriorityMailboxFactory;
pub use mailbox::DefaultPriorityMailbox;
pub use priority_mailbox_queue::PriorityMailboxQueue;
pub use sender::DefaultPriorityMailboxSender;
