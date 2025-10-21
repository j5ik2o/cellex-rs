#[cfg(test)]
mod tests;

mod local_mailbox_runtime;
mod local_mailbox_sender;
mod local_mailbox_type;
mod local_queue;
mod local_signal;
mod local_signal_wait;
mod shared;

pub use local_mailbox_runtime::LocalMailboxRuntime;
pub use local_mailbox_sender::LocalMailboxSender;
pub use local_mailbox_type::LocalMailbox;
