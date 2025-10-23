//! Backend layer traits and supporting types for stack operations.

mod push_outcome;
mod stack_backend;
mod stack_error;
mod stack_overflow_policy;
mod vec_stack_backend;

pub use push_outcome::PushOutcome;
pub use stack_backend::StackBackend;
pub use stack_error::StackError;
pub use stack_overflow_policy::StackOverflowPolicy;
pub use vec_stack_backend::VecStackBackend;
