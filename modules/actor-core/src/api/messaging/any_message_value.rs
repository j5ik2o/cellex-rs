use core::any::Any;

/// Type bound required for values stored inside [`AnyMessage`].
/// Trait bound describing values that can be erased into [`AnyMessage`].
#[cfg(target_has_atomic = "ptr")]
pub trait AnyMessageValue: Any + Send + Sync {}

/// Trait bound describing values that can be erased into [`AnyMessage`] on targets without atomic
/// pointers.
#[cfg(not(target_has_atomic = "ptr"))]
pub trait AnyMessageValue: Any {}

#[cfg(target_has_atomic = "ptr")]
impl<T> AnyMessageValue for T where T: Any + Send + Sync {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> AnyMessageValue for T where T: Any {}
