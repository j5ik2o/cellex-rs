#[cfg(target_has_atomic = "ptr")]
/// Marker trait describing factory objects safe for concurrent scheduler use.
pub trait ReceiveTimeoutSchedulerFactoryBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutSchedulerFactoryBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait for factory objects on single-threaded embedded targets.
pub trait ReceiveTimeoutSchedulerFactoryBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutSchedulerFactoryBound for T {}
