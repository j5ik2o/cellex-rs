//! Common deadline timer types.

pub mod deadline_timer_error;
pub mod deadline_timer_expired;
pub mod deadline_timer_key;
pub mod deadline_timer_key_allocator;
pub mod deadline_timer_trait;
pub mod timer_deadline;

pub use deadline_timer_error::DeadlineTimerError;
pub use deadline_timer_expired::DeadlineTimerExpired;
pub use deadline_timer_key::DeadlineTimerKey;
pub use deadline_timer_key_allocator::DeadlineTimerKeyAllocator;
pub use deadline_timer_trait::DeadlineTimer;
pub use timer_deadline::TimerDeadline;

#[cfg(test)]
mod tests;
