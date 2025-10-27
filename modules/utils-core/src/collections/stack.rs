//! Stack abstractions rebuilt for the v2 collections layer.

mod async_stack;
pub mod backend;
pub mod storage;
mod sync_stack;
#[cfg(test)]
mod tests;

pub use async_stack::AsyncStack;
pub use sync_stack::SyncStack;

/// Default shared stack alias backed by [`backend::VecStackBackend`].
pub type SharedVecStack<T> = SyncStack<T, backend::VecStackBackend<T>>;

/// Default async shared stack alias backed by [`backend::VecStackBackend`] via the sync adapter.
pub type AsyncSharedVecStack<T> = AsyncStack<T, backend::SyncAdapterStackBackend<T, backend::VecStackBackend<T>>>;
