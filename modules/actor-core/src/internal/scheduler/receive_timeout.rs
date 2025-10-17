#[cfg(target_has_atomic = "ptr")]
pub trait SchedulerBound: Send {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send> SchedulerBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait SchedulerBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerBound for T {}

#[cfg(target_has_atomic = "ptr")]
pub trait SchedulerFactoryBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> SchedulerFactoryBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait SchedulerFactoryBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerFactoryBound for T {}
