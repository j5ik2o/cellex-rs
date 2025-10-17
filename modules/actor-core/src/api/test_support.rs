//! Test support exports for mailbox-related components.
//!
//! Re-exports helpers that simplify constructing mailbox runtimes and signals in unit tests.
mod common;
mod shared_backend_handle;
mod test_mailbox_runtime;
mod test_signal;
mod test_signal_state;
mod test_signal_wait;
#[cfg(test)]
mod tests;

pub use common::TestQueue;
pub use shared_backend_handle::SharedBackendHandle;
pub use test_mailbox_runtime::TestMailboxRuntime;
pub use test_signal::TestSignal;
pub use test_signal_state::TestSignalState;
pub use test_signal_wait::TestSignalWait;
