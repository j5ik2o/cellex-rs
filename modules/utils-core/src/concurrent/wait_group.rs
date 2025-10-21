//! Wait-group primitives.

pub mod wait_group_backend;
pub mod wait_group_struct;

pub use wait_group_backend::WaitGroupBackend;
pub use wait_group_struct::WaitGroup;
