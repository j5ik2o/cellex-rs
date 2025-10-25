#[cfg(test)]
mod tests;

mod local_mailbox_factory;
mod local_mailbox_sender;
mod local_mailbox_type;
#[cfg(not(feature = "queue-v2"))]
mod local_queue;
mod local_signal;
mod local_signal_wait;
mod shared;

pub use local_mailbox_factory::LocalMailboxFactory;
pub use local_mailbox_sender::LocalMailboxSender;
pub use local_mailbox_type::LocalMailbox;
