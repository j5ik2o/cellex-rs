//! Test support exports for mailbox-related components.
//!
//! Re-exports helpers that simplify constructing mailbox factories and signals in unit tests.
mod common;
mod test_mailbox_factory;
mod test_signal;
mod test_signal_state;
mod test_signal_wait;
#[cfg(test)]
mod tests;

pub use common::TestQueue;
pub use test_mailbox_factory::TestMailboxFactory;
pub use test_signal::TestSignal;
pub use test_signal_state::TestSignalState;
pub use test_signal_wait::TestSignalWait;
