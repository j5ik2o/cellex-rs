#[cfg(target_has_atomic = "ptr")]
/// Marker trait describing factory objects safe for concurrent scheduler use.
pub trait SchedulerFactoryBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> SchedulerFactoryBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait for factory objects on single-threaded embedded targets.
pub trait SchedulerFactoryBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerFactoryBound for T {}
