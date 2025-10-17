mod error;
mod futures;
mod shared;

pub use error::AskError;
pub use futures::{ask_with_timeout, AskFuture, AskTimeoutFuture};

/// Result alias used by `ask` helpers.
pub type AskResult<T> = Result<T, AskError>;

pub(crate) use futures::create_ask_handles;
