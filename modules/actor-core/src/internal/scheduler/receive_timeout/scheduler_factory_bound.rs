#[cfg(target_has_atomic = "ptr")]
pub trait SchedulerFactoryBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> SchedulerFactoryBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait SchedulerFactoryBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerFactoryBound for T {}
