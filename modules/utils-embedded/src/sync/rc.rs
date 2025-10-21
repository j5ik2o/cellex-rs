#![allow(clippy::disallowed_types)]

mod rc_shared;
mod rc_state_cell;

pub use rc_shared::RcShared;
pub use rc_state_cell::RcStateCell;

#[cfg(test)]
mod tests;
