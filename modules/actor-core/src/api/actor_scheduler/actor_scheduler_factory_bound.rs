#[cfg(target_has_atomic = "ptr")]
/// Marker trait describing factory objects safe for concurrent scheduler use.
pub trait ActorSchedulerFactoryBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ActorSchedulerFactoryBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait for factory objects on single-threaded embedded targets.
pub trait ActorSchedulerFactoryBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ActorSchedulerFactoryBound for T {}
