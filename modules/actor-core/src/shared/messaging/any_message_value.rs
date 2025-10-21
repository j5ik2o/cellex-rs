//! Trait bound for values storable inside `AnyMessage`.

use core::any::Any;

use cellex_utils_core_rs::sync::SharedBound;

/// Trait bound required for values stored inside [`AnyMessage`](super::AnyMessage).
pub trait AnyMessageValue: Any + SharedBound {}

impl<T> AnyMessageValue for T where T: Any + SharedBound {}
