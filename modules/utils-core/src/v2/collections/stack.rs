//! Stack abstractions rebuilt for the v2 collections layer.

pub mod backend;
pub mod facade;
mod storage;

pub use backend::{PushOutcome, StackBackend, StackError, StackOverflowPolicy, VecStackBackend};
pub use facade::{AsyncStack, Stack};
pub use storage::{StackStorage, VecStackStorage};

/// Default shared stack alias backed by [`VecStackBackend`].
pub type SharedVecStack<T> = facade::Stack<T, VecStackBackend<T>>;
