//! Stack facade exposed to collection users.

mod async_stack;
mod stack;
pub use async_stack::AsyncStack;
pub use stack::Stack;

#[cfg(test)]
mod tests;
