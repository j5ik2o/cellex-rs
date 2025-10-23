//! Backend layer traits and supporting enums for queue operations.

mod offer_outcome;
mod overflow_policy;
mod priority_backend;
mod queue_backend;
mod queue_error;

pub use offer_outcome::OfferOutcome;
pub use overflow_policy::OverflowPolicy;
pub use priority_backend::PriorityBackend;
pub use queue_backend::QueueBackend;
pub use queue_error::QueueError;
