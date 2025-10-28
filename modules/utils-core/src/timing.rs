//! Module that consolidates timer-related abstractions.
//!
//! Re-exports minimal APIs referenced from core for common use by time-triggered features such as
//! `ReceiveTimeout`.

pub mod deadline_timer;
