//! Storage layer abstractions for stack backends.

mod stack_storage;
mod vec_stack_storage;

#[allow(unused_imports)]
pub use stack_storage::StackStorage;
#[allow(unused_imports)]
pub use vec_stack_storage::VecStackStorage;
