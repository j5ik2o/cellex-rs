//! Timer utilities bundled for the `std` runtime.
//!
//! Currently only re-exports `TokioDeadlineTimer`, which powers higher-level APIs such as
//! `ReceiveTimeout`.

mod tokio_deadline_timer;

pub use tokio_deadline_timer::TokioDeadlineTimer;
