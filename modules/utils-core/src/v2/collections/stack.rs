//! Stack abstractions rebuilt for the v2 collections layer.

mod async_stack;
pub mod backend;
mod stack_api;
mod storage;
#[cfg(test)]
mod tests;

pub use async_stack::AsyncStack;
pub use backend::{
  AsyncStackBackend, PushOutcome, StackBackend, StackError, StackOverflowPolicy, SyncAdapterStackBackend,
  VecStackBackend,
};
pub use stack_api::Stack;
pub use storage::{StackStorage, VecStackStorage};

/// Default shared stack alias backed by [`VecStackBackend`].
pub type SharedVecStack<T> = Stack<T, VecStackBackend<T>>;

/// Default async shared stack alias backed by [`VecStackBackend`] via the sync adapter.
pub type AsyncSharedVecStack<T> = AsyncStack<T, SyncAdapterStackBackend<T, VecStackBackend<T>>>;
