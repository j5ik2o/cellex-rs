//! Shared ownership utilities.

pub mod send_bound;
pub mod shared_bound;
pub mod shared_dyn;
pub mod shared_trait;

pub use send_bound::SendBound;
pub use shared_bound::SharedBound;
pub use shared_dyn::SharedDyn;
pub use shared_trait::Shared;

#[cfg(test)]
mod tests;
