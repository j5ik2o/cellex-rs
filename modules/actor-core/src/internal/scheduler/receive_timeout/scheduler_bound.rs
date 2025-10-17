#[cfg(target_has_atomic = "ptr")]
/// Marker trait ensuring scheduler components are `Send` on targets with pointer atomics.
pub trait SchedulerBound: Send {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send> SchedulerBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait used on single-threaded targets without pointer atomics.
pub trait SchedulerBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerBound for T {}
