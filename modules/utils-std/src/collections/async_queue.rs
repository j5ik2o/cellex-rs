//! Async queue adapters for std + Tokio environments.

mod tokio_bounded_mpsc_backend;

#[cfg(test)]
mod tests;

pub use tokio_bounded_mpsc_backend::{make_tokio_mpsc_queue, TokioBoundedMpscBackend, TokioMpscQueue};
