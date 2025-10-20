use core::any::Any;

use cellex_utils_core_rs::sync::SharedBound;

/// Trait bound required for values stored inside [`AnyMessage`](crate::api::messaging::AnyMessage).
pub trait AnyMessageValue: Any + SharedBound {}

impl<T> AnyMessageValue for T where T: Any + SharedBound {}
