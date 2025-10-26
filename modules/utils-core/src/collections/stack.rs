mod buffer;
mod r#impl;
/// Stack trait definitions.
pub mod traits;

/// Type alias for stack buffer implementation.
pub type StackBuffer<T> = buffer::StackBuffer<T>;
/// Type alias for stack error type.
pub type StackError<T> = buffer::StackError<T>;
/// Type alias for stack implementation.
pub type Stack<H, T> = r#impl::Stack<H, T>;
